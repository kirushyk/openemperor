//! The 3D battlefield: terrain and camera, projected on the CPU.
//!
//! Terrain is drawn back to front, so it needs no depth buffer and the graphics backends
//! still only receive colored triangles.
//!
//! The map is a square grid turned 45 degrees into a diamond. Grid coordinates (u, v) run
//! 0..=MAP_CELLS: north is (0, 0), east (MAP_CELLS, 0), west (0, MAP_CELLS) and south
//! (MAP_CELLS, MAP_CELLS). World x points east, y up and z south.

use crate::fog::Fog;
use crate::planet::fractal_noise;
use crate::platform::Vertex;
use std::sync::OnceLock;

pub const MAP_CELLS: usize = 128;
const GRID_LAST: f32 = MAP_CELLS as f32;
pub const GRID_POINTS: usize = (MAP_CELLS + 1) * (MAP_CELLS + 1);
/// World units per grid step along each world axis.
const CELL: f32 = 0.5;
/// How far ground rises above sand, in world units.
const GROUND_HEIGHT: f32 = 0.7;
/// Noise above this is the upper terrain, below it the lower.
const GROUND_THRESHOLD: f32 = 0.54;
/// More land where the lower terrain cannot be walked on.
const LAND_THRESHOLD: f32 = 0.5;

const SAND: [f32; 3] = [0.78, 0.64, 0.40];
const GROUND: [f32; 3] = [0.40, 0.30, 0.21];

/// Which world a map is on, which sets how its two heights look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Planet {
    /// Sand and rocky ground.
    A,
    /// Home of faction A: water and grass.
    C,
    /// Home of faction O: ice and snow.
    D,
    /// Home of faction H: lava and the same ground as planet A.
    G,
    /// A prison planet, nobody's home.
    S,
    /// Home of the mini faction T.
    T,
}

impl Planet {
    fn lower_color(self) -> [f32; 3] {
        match self {
            Planet::A => SAND,
            Planet::C => [0.13, 0.36, 0.60],
            Planet::D => [0.66, 0.82, 0.93],
            Planet::G => [0.90, 0.28, 0.05],
            Planet::S => [0.44, 0.42, 0.40],
            Planet::T => [0.47, 0.50, 0.30],
        }
    }

    fn upper_color(self) -> [f32; 3] {
        match self {
            Planet::A | Planet::G => GROUND,
            Planet::C => [0.24, 0.50, 0.20],
            Planet::D => [0.96, 0.97, 1.00],
            Planet::S => [0.24, 0.23, 0.25],
            Planet::T => [0.30, 0.25, 0.30],
        }
    }

    /// Water and lava: nothing can stand on the lower terrain.
    fn lower_is_solid(self) -> bool {
        !matches!(self, Planet::C | Planet::G)
    }
}

/// The planet of each map in `game::MAPS`.
pub const MAP_PLANETS: [Planet; crate::game::MAPS.len()] = [
    Planet::A,
    Planet::A,
    Planet::A,
    Planet::A,
    Planet::A,
    Planet::C,
    Planet::D,
    Planet::G,
    Planet::S,
    Planet::T,
];
/// How strongly the surface grain varies the color, either way.
const GRAIN: f32 = 0.22;

/// The first half of zooming moves the camera between two fixed positions. The second half
/// stays at the bottom position and narrows the field of view. Top: high and looking almost
/// straight down.
const TOP_HEIGHT: f32 = 30.0;
const TOP_PITCH_DEGREES: f32 = 75.0;
/// Bottom: low and looking almost forward, 30 degrees down from the ground.
const BOTTOM_HEIGHT: f32 = 8.0;
const BOTTOM_PITCH_DEGREES: f32 = 30.0;
/// How much the field of view narrows at the very end of the zoom.
const MAX_LENS: f32 = 2.0;
/// Share of the whole zoom per wheel notch.
const ZOOM_PER_NOTCH: f32 = 0.05;
/// How quickly the camera catches up with the wheel; higher is snappier.
const ZOOM_EASING: f32 = 10.0;
/// Vertical field of view, as the tangent of half its angle (about 40 degrees in total).
const HALF_FOV_TAN: f32 = 0.364;
const NEAR: f32 = 1.0;

/// Pixels from the screen edge that start scrolling, on a 1080-pixel-tall screen.
const EDGE_MARGIN: f32 = 4.0;
/// World units per second, per point of the Scroll Rate option.
const SCROLL_PER_RATE: f32 = 15.0;

/// World z of the map's middle; its x is 0.
pub const MAP_MIDDLE_Z: f32 = MAP_CELLS as f32 * CELL;

/// How many destroyed buildings each map tries to place.
const RUIN_COUNT: usize = 24;

/// One block of a destroyed building: a stretch of broken wall, or rubble.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuinPiece {
    pub x: f32,
    /// Height of the ground under it.
    pub y: f32,
    pub z: f32,
    pub half_x: f32,
    pub half_z: f32,
    pub height: f32,
    /// How light its concrete is, so that pieces do not all look alike.
    pub shade: f32,
}

pub struct Terrain {
    planet: Planet,
    is_ground: Vec<bool>,
    colors: Vec<[f32; 3]>,
    ruins: Vec<RuinPiece>,
}

/// Xorshift: repeatable, and it needs no dependency.
fn xorshift(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

/// A repeatable random number from 0 to 1.
pub fn next_random(state: &mut u32) -> f32 {
    (xorshift(state) & 0xFFFF) as f32 / 65535.0
}

/// The remains of one square building: what is left of its four walls, in broken stretches
/// of uneven height, and some rubble inside.
fn add_ruin(pieces: &mut Vec<RuinPiece>, terrain: &Terrain, x: f32, z: f32, random: &mut u32) {
    const THICKNESS: f32 = 0.09;
    const STRETCHES: usize = 3;
    let half = 1.0 + next_random(random) * 1.0;
    let y = terrain.height_at(x, z);
    for side in 0..4 {
        for stretch in 0..STRETCHES {
            // About a third of the wall has fallen altogether.
            if next_random(random) < 0.35 {
                continue;
            }
            let along = ((stretch as f32 + 0.5) / STRETCHES as f32 * 2.0 - 1.0) * half;
            let half_length = half / STRETCHES as f32;
            let (dx, dz, half_x, half_z) = match side {
                0 => (along, -half, half_length, THICKNESS),
                1 => (along, half, half_length, THICKNESS),
                2 => (-half, along, THICKNESS, half_length),
                _ => (half, along, THICKNESS, half_length),
            };
            pieces.push(RuinPiece {
                x: x + dx,
                y,
                z: z + dz,
                half_x,
                half_z,
                height: 0.2 + next_random(random) * 1.1,
                shade: 0.7 + next_random(random) * 0.3,
            });
        }
    }
    for _ in 0..4 {
        let size = 0.1 + next_random(random) * 0.18;
        pieces.push(RuinPiece {
            x: x + (next_random(random) * 2.0 - 1.0) * half * 0.8,
            y,
            z: z + (next_random(random) * 2.0 - 1.0) * half * 0.8,
            half_x: size,
            half_z: size,
            height: size * 1.2,
            shade: 0.5 + next_random(random) * 0.3,
        });
    }
}

pub fn grid_index(u: usize, v: usize) -> usize {
    v * (MAP_CELLS + 1) + u
}

impl Terrain {
    fn generate(map: usize) -> Self {
        let mut is_ground = Vec::with_capacity(GRID_POINTS);
        let mut colors = Vec::with_capacity(GRID_POINTS);
        let seed = 101 + map as u32 * 13;
        let planet = MAP_PLANETS[map];
        let threshold = if planet.lower_is_solid() {
            GROUND_THRESHOLD
        } else {
            LAND_THRESHOLD
        };
        for v in 0..=MAP_CELLS {
            for u in 0..=MAP_CELLS {
                let (fu, fv) = (u as f32 / MAP_CELLS as f32, v as f32 / MAP_CELLS as f32);
                let height = fractal_noise([fu * 2.5, fv * 2.5, 0.5], seed);
                let (x, z) = grid_to_world(u as f32, v as f32);
                let ground = height > threshold
                    || (!planet.lower_is_solid() && crate::units::needs_land(x, z));
                let grain = fractal_noise([fu * 30.0, fv * 30.0, 2.5], seed + 500) - 0.5;
                let base = if ground {
                    planet.upper_color()
                } else {
                    planet.lower_color()
                };
                is_ground.push(ground);
                colors.push(base.map(|c| c * (1.0 + grain * 2.0 * GRAIN)));
            }
        }
        let mut terrain = Terrain {
            planet,
            is_ground,
            colors,
            ruins: Vec::new(),
        };
        let mut random = (seed + 1).wrapping_mul(0x9E37_79B1) | 1;
        let mut pieces = Vec::new();
        // Inset from the map's edges, so that no ruin hangs over them.
        let (inset, span) = (8.0, MAP_CELLS as f32 - 16.0);
        // Land is scarcer where the lower terrain cannot hold anything, so look harder there.
        let tries = if planet.lower_is_solid() {
            RUIN_COUNT
        } else {
            RUIN_COUNT * 8
        };
        let mut placed = 0;
        for _ in 0..tries {
            let u = inset + next_random(&mut random) * span;
            let v = inset + next_random(&mut random) * span;
            let (x, z) = grid_to_world(u, v);
            if placed < RUIN_COUNT && crate::units::keeps_clear(x, z) && terrain.can_hold_ruin(x, z)
            {
                add_ruin(&mut pieces, &terrain, x, z, &mut random);
                placed += 1;
            }
        }
        terrain.ruins = pieces;
        terrain
    }

    /// Nothing stands in water or lava, so there the whole site must be on land.
    fn can_hold_ruin(&self, x: f32, z: f32) -> bool {
        const SITE: f32 = 2.4;
        self.planet.lower_is_solid()
            || [
                (-SITE, -SITE),
                (SITE, -SITE),
                (SITE, SITE),
                (-SITE, SITE),
                (0.0, -SITE),
                (SITE, 0.0),
                (0.0, SITE),
                (-SITE, 0.0),
                (0.0, 0.0),
            ]
            .iter()
            .all(|(dx, dz)| self.height_at(x + dx, z + dz) >= GROUND_HEIGHT - 0.001)
    }

    pub fn lower_color(&self) -> [f32; 3] {
        self.planet.lower_color()
    }

    pub fn upper_color(&self) -> [f32; 3] {
        self.planet.upper_color()
    }

    /// Whether a unit may stand here: on the map, and not in water or lava.
    pub fn can_walk(&self, x: f32, z: f32) -> bool {
        self.on_map(x, z)
            && (self.planet.lower_is_solid() || self.height_at(x, z) >= GROUND_HEIGHT * 0.5)
    }

    pub fn on_map(&self, x: f32, z: f32) -> bool {
        let (u, v) = world_to_grid(x, z);
        (0.0..=GRID_LAST).contains(&u) && (0.0..=GRID_LAST).contains(&v)
    }

    /// Flat sand holding only the given ruins, for testing how units meet them.
    #[cfg(test)]
    pub fn open_with_ruins(ruins: Vec<RuinPiece>) -> Self {
        Terrain {
            planet: Planet::A,
            is_ground: vec![false; GRID_POINTS],
            colors: vec![SAND; GRID_POINTS],
            ruins,
        }
    }

    /// The destroyed buildings on this map.
    /// A point anywhere on the map.
    pub fn random_point(&self, random: &mut u32) -> (f32, f32) {
        let (u, v) = (next_random(random), next_random(random));
        grid_to_world(u * GRID_LAST, v * GRID_LAST)
    }

    pub fn ruins(&self) -> &[RuinPiece] {
        &self.ruins
    }

    pub fn is_ground(&self, u: usize, v: usize) -> bool {
        self.is_ground[grid_index(u, v)]
    }

    /// The terrain's height under a world point, following its slopes.
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let (u, v) = world_to_grid(x, z);
        let (u, v) = (u.clamp(0.0, GRID_LAST), v.clamp(0.0, GRID_LAST));
        let (u0, v0) = (
            u.floor().min(GRID_LAST - 1.0),
            v.floor().min(GRID_LAST - 1.0),
        );
        let (tu, tv) = (u - u0, v - v0);
        let at = |du: usize, dv: usize| {
            let raised = self.is_ground(u0 as usize + du, v0 as usize + dv);
            if raised { GROUND_HEIGHT } else { 0.0 }
        };
        let near = at(0, 0) * (1.0 - tu) + at(1, 0) * tu;
        let far = at(0, 1) * (1.0 - tu) + at(1, 1) * tu;
        near * (1.0 - tv) + far * tv
    }
}

/// The terrain of one map, generated the first time it is needed.
pub fn terrain(map: usize) -> &'static Terrain {
    static TERRAINS: [OnceLock<Terrain>; crate::game::MAPS.len()] =
        [const { OnceLock::new() }; crate::game::MAPS.len()];
    TERRAINS[map].get_or_init(|| Terrain::generate(map))
}

pub fn grid_to_world(u: f32, v: f32) -> (f32, f32) {
    ((u - v) * CELL, (u + v) * CELL)
}

pub fn world_to_grid(x: f32, z: f32) -> (f32, f32) {
    ((z + x) / (2.0 * CELL), (z - x) / (2.0 * CELL))
}

/// The ground point the camera looks at, and how far it has zoomed: 0 is the top position,
/// 0.5 the bottom position, and 1 the bottom position with the field of view narrowed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub x: f32,
    pub z: f32,
    pub zoom: f32,
    zoom_target: f32,
}

impl Camera {
    /// Looking at the middle of the map from the top position.
    pub fn centered() -> Self {
        let middle = MAP_CELLS as f32 / 2.0;
        let (x, z) = grid_to_world(middle, middle);
        Camera {
            x,
            z,
            zoom: 0.0,
            zoom_target: 0.0,
        }
    }

    /// Positive notches go down towards the bottom position.
    pub fn zoom_by(&mut self, notches: f32) {
        self.zoom_target = (self.zoom_target + notches * ZOOM_PER_NOTCH).clamp(0.0, 1.0);
    }

    /// One frame of the camera gliding towards where the wheel left it.
    pub fn ease_zoom(&mut self, dt: f32) {
        self.zoom += (self.zoom_target - self.zoom) * (1.0 - (-ZOOM_EASING * dt).exp());
        if (self.zoom_target - self.zoom).abs() < 0.001 {
            self.zoom = self.zoom_target;
        }
    }

    /// Centers the view on a world point, kept on the map.
    pub fn look_at(&mut self, x: f32, z: f32) {
        (self.x, self.z) = (x, z);
        self.move_by(0.0, 0.0);
    }

    /// Moves the target, sliding along the map's edges rather than leaving the diamond.
    fn move_by(&mut self, dx: f32, dz: f32) {
        let (u, v) = world_to_grid(self.x + dx, self.z + dz);
        (self.x, self.z) = grid_to_world(u.clamp(0.0, GRID_LAST), v.clamp(0.0, GRID_LAST));
    }

    /// One frame of edge scrolling. Slower when low, so the ground crosses the screen at
    /// about the same pace from either position.
    pub fn scroll(&mut self, direction: (f32, f32), scroll_rate: i32, dt: f32) {
        let pace = self.altitude() / TOP_HEIGHT / self.lens();
        let distance = SCROLL_PER_RATE * scroll_rate as f32 * pace * dt;
        let (dx, dz) = (direction.0 * distance, direction.1 * distance);
        if dx != 0.0 && dz != 0.0 {
            self.move_diagonally(dx, dz);
        } else {
            self.move_by(dx, dz);
        }
    }

    /// A push towards a screen corner runs along one grid axis, straight at a diamond edge.
    /// Once there it carries on along that edge to a real map corner, where it finally
    /// stops: the nearer corner, or the east or west one from the exact middle of the edge.
    fn move_diagonally(&mut self, dx: f32, dz: f32) {
        let (u, v) = world_to_grid(self.x, self.z);
        let (du, dv) = world_to_grid(dx, dz);
        let blocked = |position: f32, step: f32| {
            (position >= GRID_LAST && step > 0.0) || (position <= 0.0 && step < 0.0)
        };
        let slide = |other: f32, step: f32| {
            let middle = GRID_LAST / 2.0;
            let end = if other < middle {
                0.0
            } else if other > middle {
                GRID_LAST
            } else if step > 0.0 {
                0.0
            } else {
                GRID_LAST
            };
            let distance = step.abs();
            if end > other {
                (other + distance).min(end)
            } else {
                (other - distance).max(end)
            }
        };
        let (u, v) = if du.abs() >= dv.abs() && blocked(u, du) {
            (u, slide(v, du))
        } else if dv.abs() > du.abs() && blocked(v, dv) {
            (slide(u, dv), v)
        } else {
            (u + du, v + dv)
        };
        (self.x, self.z) = grid_to_world(u.clamp(0.0, GRID_LAST), v.clamp(0.0, GRID_LAST));
    }

    /// How far between the two positions: 0 at the top, 1 from the bottom position onward.
    fn descent(self) -> f32 {
        (self.zoom * 2.0).min(1.0)
    }

    fn altitude(self) -> f32 {
        TOP_HEIGHT + (BOTTOM_HEIGHT - TOP_HEIGHT) * self.descent()
    }

    /// Magnification from narrowing the field of view: 1 until the bottom position.
    fn lens(self) -> f32 {
        1.0 + (MAX_LENS - 1.0) * (self.zoom * 2.0 - 1.0).max(0.0)
    }
}

/// A camera pose ready for projecting, for one frame on one screen.
pub struct View {
    position: [f32; 3],
    /// Unit vectors, with no x part: the camera never turns sideways.
    forward: [f32; 2],
    up: [f32; 2],
    focal: f32,
    center: (f32, f32),
}

impl View {
    pub fn new(camera: Camera, width: f32, height: f32) -> Self {
        let degrees =
            TOP_PITCH_DEGREES + (BOTTOM_PITCH_DEGREES - TOP_PITCH_DEGREES) * camera.descent();
        let (sin, cos) = degrees.to_radians().sin_cos();
        let altitude = camera.altitude();
        View {
            position: [camera.x, altitude, camera.z + altitude * cos / sin],
            forward: [-sin, -cos],
            up: [cos, -sin],
            focal: height / (2.0 * HALF_FOV_TAN) * camera.lens(),
            center: (width / 2.0, height / 2.0),
        }
    }

    /// Where the camera is in the world, as (x, z).
    pub fn position_on_ground(&self) -> (f32, f32) {
        (self.position[0], self.position[2])
    }

    /// A world point seen from the camera: how far to the side, how far up, and how deep.
    fn relative(&self, x: f32, y: f32, z: f32) -> (f32, f32, f32) {
        let [px, py, pz] = self.position;
        let (side, rel_y, rel_z) = (x - px, y - py, z - pz);
        let up = rel_y * self.up[0] + rel_z * self.up[1];
        (side, up, rel_y * self.forward[0] + rel_z * self.forward[1])
    }

    fn to_screen(&self, side: f32, up: f32, depth: f32) -> (f32, f32) {
        (
            self.center.0 + self.focal * side / depth,
            self.center.1 - self.focal * up / depth,
        )
    }

    /// A world point's place on screen; `None` if it is behind the camera.
    pub fn project(&self, x: f32, y: f32, z: f32) -> Option<(f32, f32)> {
        let (side, up, depth) = self.relative(x, y, z);
        (depth >= NEAR).then(|| self.to_screen(side, up, depth))
    }

    /// A straight line on the sand plane as it appears on screen, cut where it passes behind
    /// the camera; `None` if all of it is behind.
    pub fn project_ground_line(
        &self,
        a: (f32, f32),
        b: (f32, f32),
    ) -> Option<((f32, f32), (f32, f32))> {
        let depth = |point: (f32, f32)| self.relative(point.0, 0.0, point.1).2;
        let (depth_a, depth_b) = (depth(a), depth(b));
        // Just in front of the limit, so that rounding cannot put a cut end behind it.
        let limit = NEAR + 0.01;
        if depth_a < limit && depth_b < limit {
            return None;
        }
        let cut = |behind: (f32, f32), ahead: (f32, f32), d_behind: f32, d_ahead: f32| {
            let t = (limit - d_behind) / (d_ahead - d_behind);
            (
                behind.0 + (ahead.0 - behind.0) * t,
                behind.1 + (ahead.1 - behind.1) * t,
            )
        };
        let from = if depth_a < limit {
            cut(a, b, depth_a, depth_b)
        } else {
            a
        };
        let to = if depth_b < limit {
            cut(b, a, depth_b, depth_a)
        } else {
            b
        };
        Some((
            self.project(from.0, 0.0, from.1)?,
            self.project(to.0, 0.0, to.1)?,
        ))
    }

    /// Like `project`, but a point behind the camera lands far off the screen's bottom edge
    /// on its own side, which is enough for drawing lines towards it.
    pub fn project_clamped(&self, x: f32, y: f32, z: f32) -> (f32, f32) {
        let (side, up, depth) = self.relative(x, y, z);
        self.to_screen(side, up, depth.max(NEAR))
    }

    /// The world (x, z) on the sand plane under a screen point; `None` above the horizon.
    pub fn ground_at(&self, screen_x: f32, screen_y: f32) -> Option<(f32, f32)> {
        let side = (screen_x - self.center.0) / self.focal;
        let lift = (self.center.1 - screen_y) / self.focal;
        let ray_y = self.forward[0] + lift * self.up[0];
        let ray_z = self.forward[1] + lift * self.up[1];
        if ray_y >= 0.0 {
            return None;
        }
        let [px, py, pz] = self.position;
        let t = -py / ray_y;
        Some((px + side * t, pz + ray_z * t))
    }

    /// Where the screen's corners fall on the ground, clockwise from the top left. The far
    /// edge is the wider one, so this is the view's true trapezium rather than a rectangle.
    pub fn ground_corners(&self) -> Option<[(f32, f32); 4]> {
        let (right, bottom) = (self.center.0 * 2.0, self.center.1 * 2.0);
        Some([
            self.ground_at(0.0, 0.0)?,
            self.ground_at(right, 0.0)?,
            self.ground_at(right, bottom)?,
            self.ground_at(0.0, bottom)?,
        ])
    }
}

/// A world point's place on a minimap square whose inscribed diamond is the whole map.
pub fn world_to_minimap((x, y, size): (f32, f32, f32), world_x: f32, world_z: f32) -> (f32, f32) {
    (
        x + size / 2.0 + world_x / MAP_MIDDLE_Z * size / 2.0,
        y + world_z / MAP_MIDDLE_Z * size / 2.0,
    )
}

/// The world point under a minimap position, or `None` outside the minimap's square.
pub fn minimap_to_world((x, y, size): (f32, f32, f32), px: f32, py: f32) -> Option<(f32, f32)> {
    if px < x || px >= x + size || py < y || py >= y + size {
        return None;
    }
    Some((
        (px - (x + size / 2.0)) / (size / 2.0) * MAP_MIDDLE_Z,
        (py - y) / (size / 2.0) * MAP_MIDDLE_Z,
    ))
}

/// The part of a segment inside a square, if any.
pub fn clip_to_square(
    a: (f32, f32),
    b: (f32, f32),
    (x, y, size): (f32, f32, f32),
) -> Option<((f32, f32), (f32, f32))> {
    clip_to_rect(a, b, (x, y, size, size))
}

/// The part of a segment inside a rectangle, if any (Liang-Barsky).
pub fn clip_to_rect(
    a: (f32, f32),
    b: (f32, f32),
    (x, y, width, height): (f32, f32, f32, f32),
) -> Option<((f32, f32), (f32, f32))> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut from, mut to) = (0.0_f32, 1.0_f32);
    for (delta, distance) in [
        (-dx, a.0 - x),
        (dx, x + width - a.0),
        (-dy, a.1 - y),
        (dy, y + height - a.1),
    ] {
        if delta == 0.0 {
            if distance < 0.0 {
                return None;
            }
        } else if delta < 0.0 {
            from = from.max(distance / delta);
        } else {
            to = to.min(distance / delta);
        }
    }
    (from <= to).then_some((
        (a.0 + dx * from, a.1 + dy * from),
        (a.0 + dx * to, a.1 + dy * to),
    ))
}

/// Whether all these screen points lie beyond one side of a screen, so that nothing between
/// them can show.
pub fn all_off_screen(points: &[(f32, f32)], width: f32, height: f32) -> bool {
    points.iter().all(|p| p.0 < 0.0)
        || points.iter().all(|p| p.0 > width)
        || points.iter().all(|p| p.1 < 0.0)
        || points.iter().all(|p| p.1 > height)
}

/// Which way the cursor pushes the camera, with length 1 or 0. Corners push diagonally.
pub fn edge_direction(mouse: (f32, f32), width: f32, height: f32) -> (f32, f32) {
    let margin = EDGE_MARGIN * crate::game::ui_scale(height);
    let push = |position: f32, size: f32| {
        if position <= margin {
            -1.0
        } else if position >= size - 1.0 - margin {
            1.0
        } else {
            0.0
        }
    };
    let (dx, dz) = (push(mouse.0, width), push(mouse.1, height));
    let length = f32::hypot(dx, dz);
    if length > 0.0 {
        (dx / length, dz / length)
    } else {
        (0.0, 0.0)
    }
}

/// Appends the visible terrain, farthest first. Both grid axes point towards the camera,
/// so plain nested loops give that order.
pub fn draw(
    out: &mut Vec<Vertex>,
    terrain: &Terrain,
    fog: Option<&Fog>,
    camera: Camera,
    width: f32,
    height: f32,
) {
    let view = View::new(camera, width, height);
    let point = |u: usize, v: usize| {
        let index = grid_index(u, v);
        let (x, z) = grid_to_world(u as f32, v as f32);
        let y = if terrain.is_ground[index] {
            GROUND_HEIGHT
        } else {
            0.0
        };
        let shade = fog.map_or(1.0, |fog| fog.at_grid(u, v).shade());
        let [r, g, b] = terrain.colors[index].map(|channel| channel * shade);
        view.project(x, y, z)
            .map(|point| Vertex::new(point, [r, g, b, 1.0]))
    };
    for u in 0..MAP_CELLS {
        for v in 0..MAP_CELLS {
            let corners = [
                point(u, v),
                point(u + 1, v),
                point(u + 1, v + 1),
                point(u, v + 1),
            ];
            let [Some(a), Some(b), Some(c), Some(d)] = corners else {
                continue;
            };
            if !all_off_screen(&[a, b, c, d].map(|p| (p.x, p.y)), width, height) {
                out.extend([a, b, c, a, c, d]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f32 = 2880.0;
    const H: f32 = 1800.0;
    const LAST: f32 = MAP_CELLS as f32;

    fn screen(camera: Camera, u: f32, v: f32, y: f32) -> (f32, f32) {
        let (x, z) = grid_to_world(u, v);
        View::new(camera, W, H).project(x, y, z).unwrap()
    }

    fn at_zoom(zoom: f32) -> Camera {
        let mut camera = Camera::centered();
        camera.zoom = zoom;
        camera.zoom_target = zoom;
        camera
    }

    /// The camera's pitch below the horizon, in degrees.
    fn pitch(camera: Camera) -> f32 {
        let view = View::new(camera, W, H);
        (-view.forward[0]).atan2(-view.forward[1]).to_degrees()
    }

    #[test]
    fn zoom_has_two_fixed_positions_with_the_requested_tilt() {
        assert!(
            (pitch(at_zoom(0.0)) - 75.0).abs() < 0.01,
            "top looks almost straight down"
        );
        assert!(
            (pitch(at_zoom(0.5)) - 30.0).abs() < 0.01,
            "bottom is 30 degrees from the ground"
        );
        assert!(
            View::new(at_zoom(0.5), W, H).position[1] < View::new(at_zoom(0.0), W, H).position[1]
        );
    }

    #[test]
    fn the_first_half_moves_the_camera_and_keeps_the_field_of_view() {
        let wide = View::new(at_zoom(0.0), W, H).focal;
        for zoom in [0.0, 0.2, 0.4, 0.5] {
            assert_eq!(View::new(at_zoom(zoom), W, H).focal, wide, "at {zoom}");
        }
        assert!(
            pitch(at_zoom(0.25)) > pitch(at_zoom(0.5)),
            "still moving at 0.25"
        );
    }

    #[test]
    fn the_second_half_stays_put_and_narrows_the_field_of_view_up_to_2x() {
        let (bottom, end) = (View::new(at_zoom(0.5), W, H), View::new(at_zoom(1.0), W, H));
        assert_eq!(end.position, bottom.position, "the camera does not move");
        assert_eq!(end.forward, bottom.forward, "or turn");
        assert!(
            (end.focal / bottom.focal - 2.0).abs() < 1e-4,
            "2x at the end"
        );
        let middle = View::new(at_zoom(0.75), W, H).focal / bottom.focal;
        assert!(middle > 1.0 && middle < 2.0, "gradual: {middle}");
    }

    #[test]
    fn zooming_keeps_the_same_ground_point_at_screen_center() {
        for zoom in [0.0, 0.3, 0.5, 0.7, 1.0] {
            let camera = at_zoom(zoom);
            let (x, y) = View::new(camera, W, H)
                .project(camera.x, 0.0, camera.z)
                .unwrap();
            assert!(
                (x - W / 2.0).abs() < 0.01 && (y - H / 2.0).abs() < 0.01,
                "at {zoom}"
            );
        }
    }

    #[test]
    fn narrowing_the_field_of_view_shrinks_the_view_outline() {
        let width = |zoom| {
            let [left, right, ..] = View::new(at_zoom(zoom), W, H).ground_corners().unwrap();
            right.0 - left.0
        };
        assert!(width(1.0) < width(0.5) * 0.6);
    }

    #[test]
    fn scrolling_slows_further_as_the_view_narrows() {
        let moved = |zoom| {
            let mut camera = at_zoom(zoom);
            camera.scroll((1.0, 0.0), 5, 0.1);
            camera.x - at_zoom(zoom).x
        };
        assert!((moved(0.5) / moved(1.0) - 2.0).abs() < 0.01);
    }

    #[test]
    fn the_wheel_moves_gradually_and_stops_at_both_positions() {
        let mut camera = Camera::centered();
        camera.zoom_by(1.0);
        assert_eq!(camera.zoom, 0.0, "nothing moves until time passes");
        camera.ease_zoom(0.016);
        assert!(
            camera.zoom > 0.0 && camera.zoom < ZOOM_PER_NOTCH,
            "glides: {}",
            camera.zoom
        );
        for _ in 0..200 {
            camera.ease_zoom(0.016);
        }
        assert_eq!(camera.zoom, ZOOM_PER_NOTCH, "arrives exactly");

        camera.zoom_by(100.0);
        for _ in 0..200 {
            camera.ease_zoom(0.016);
        }
        assert_eq!(camera.zoom, 1.0, "stops at the bottom");

        camera.zoom_by(-100.0);
        for _ in 0..200 {
            camera.ease_zoom(0.016);
        }
        assert_eq!(camera.zoom, 0.0, "stops at the top");
    }

    fn at_grid(u: f32, v: f32) -> Camera {
        let mut camera = Camera::centered();
        (camera.x, camera.z) = grid_to_world(u, v);
        camera
    }

    fn push_until_stopped(mut camera: Camera, direction: (f32, f32)) -> (f32, f32) {
        for _ in 0..4000 {
            camera.scroll(direction, 10, 0.05);
        }
        world_to_grid(camera.x, camera.z)
    }

    #[test]
    fn corner_pushes_slide_along_the_edge_to_a_true_corner() {
        let d = std::f32::consts::FRAC_1_SQRT_2;
        let (north, east, south, west) = ((0.0, 0.0), (LAST, 0.0), (LAST, LAST), (0.0, LAST));
        let middle = LAST / 2.0;
        let cases = [
            // From the center each push hits the middle of an edge and goes east or west.
            (
                "bottom right from center",
                at_grid(middle, middle),
                (d, d),
                east,
            ),
            (
                "top right from center",
                at_grid(middle, middle),
                (d, -d),
                east,
            ),
            (
                "bottom left from center",
                at_grid(middle, middle),
                (-d, d),
                west,
            ),
            (
                "top left from center",
                at_grid(middle, middle),
                (-d, -d),
                west,
            ),
            // Off the middle of an edge, the nearer corner wins.
            (
                "bottom right near south",
                at_grid(middle, LAST - 10.0),
                (d, d),
                south,
            ),
            (
                "bottom right near east",
                at_grid(middle, 10.0),
                (d, d),
                east,
            ),
            (
                "top right near north",
                at_grid(10.0, middle),
                (d, -d),
                north,
            ),
            (
                "top right near east",
                at_grid(LAST - 10.0, middle),
                (d, -d),
                east,
            ),
            (
                "bottom left near south",
                at_grid(LAST - 10.0, middle),
                (-d, d),
                south,
            ),
            (
                "bottom left near west",
                at_grid(10.0, middle),
                (-d, d),
                west,
            ),
            (
                "top left near north",
                at_grid(middle, 10.0),
                (-d, -d),
                north,
            ),
            (
                "top left near west",
                at_grid(middle, LAST - 10.0),
                (-d, -d),
                west,
            ),
        ];
        for (name, camera, direction, corner) in cases {
            let (u, v) = push_until_stopped(camera, direction);
            assert!(
                (u - corner.0).abs() < 0.01 && (v - corner.1).abs() < 0.01,
                "{name}: ended at {u}, {v}"
            );
        }
    }

    #[test]
    fn sliding_keeps_moving_at_full_speed_along_the_edge() {
        let d = std::f32::consts::FRAC_1_SQRT_2;
        let mut on_edge = at_grid(LAST, 40.0);
        let before = on_edge;
        on_edge.scroll((d, d), 5, 0.1);
        let slid = (on_edge.x - before.x).hypot(on_edge.z - before.z);

        let mut open = Camera::centered();
        let start = open;
        open.scroll((d, d), 5, 0.1);
        let free = (open.x - start.x).hypot(open.z - start.z);
        assert!(slid > 0.0, "does not stop at the edge");
        assert!((slid - free).abs() < 0.01, "{slid} vs {free}");
    }

    #[test]
    fn a_true_corner_stops_the_push_that_points_into_it() {
        let d = std::f32::consts::FRAC_1_SQRT_2;
        for (corner, direction) in [
            ((LAST, 0.0), (d, d)),
            ((LAST, 0.0), (d, -d)),
            ((0.0, LAST), (-d, d)),
            ((0.0, LAST), (-d, -d)),
            ((0.0, 0.0), (d, -d)),
            ((0.0, 0.0), (-d, -d)),
            ((LAST, LAST), (d, d)),
            ((LAST, LAST), (-d, d)),
        ] {
            let start = at_grid(corner.0, corner.1);
            let mut camera = start;
            camera.scroll(direction, 10, 0.1);
            assert_eq!(camera, start, "{corner:?} pushed by {direction:?}");
        }
    }

    #[test]
    fn scrolling_is_gentler_from_the_bottom_position() {
        let moved = |zoom| {
            let mut camera = at_zoom(zoom);
            camera.scroll((1.0, 0.0), 5, 0.1);
            camera.x - at_zoom(zoom).x
        };
        assert!(moved(1.0) < moved(0.0));
    }

    #[test]
    fn ground_corners_are_a_trapezium_wider_at_the_far_edge() {
        for zoom in [0.0, 0.5, 1.0] {
            let camera = at_zoom(zoom);
            let [top_left, top_right, bottom_right, bottom_left] = View::new(camera, W, H)
                .ground_corners()
                .expect("the view always hits the ground");
            let far_width = top_right.0 - top_left.0;
            let near_width = bottom_right.0 - bottom_left.0;
            assert!(
                far_width > near_width * 1.01,
                "at {zoom}: {far_width} vs {near_width}"
            );
            assert!(
                top_left.1 < bottom_left.1,
                "far edge is north of the near edge"
            );
            assert!((top_left.1 - top_right.1).abs() < 0.001, "edges stay level");
            assert!(
                top_left.1 < camera.z && camera.z < bottom_left.1,
                "contains the target"
            );
        }
    }

    #[test]
    fn ground_at_reverses_projection() {
        for zoom in [0.0, 0.5, 1.0] {
            let view = View::new(at_zoom(zoom), W, H);
            let (x, z) = (5.0, at_zoom(zoom).z - 3.0);
            let (sx, sy) = view.project(x, 0.0, z).unwrap();
            let (gx, gz) = view.ground_at(sx, sy).unwrap();
            assert!((gx - x).abs() < 0.01 && (gz - z).abs() < 0.01, "at {zoom}");
        }
    }

    #[test]
    fn minimap_positions_match_the_map_corners_and_round_trip() {
        let rect = (1000.0, 100.0, 400.0);
        let corner = |u, v| {
            let (x, z) = grid_to_world(u, v);
            world_to_minimap(rect, x, z)
        };
        assert_eq!(corner(0.0, 0.0), (1200.0, 100.0), "north is top middle");
        assert_eq!(
            corner(LAST, LAST),
            (1200.0, 500.0),
            "south is bottom middle"
        );
        assert_eq!(corner(LAST, 0.0), (1400.0, 300.0), "east is right middle");
        assert_eq!(corner(0.0, LAST), (1000.0, 300.0), "west is left middle");

        let (x, z) = minimap_to_world(rect, 1250.0, 260.0).unwrap();
        let back = world_to_minimap(rect, x, z);
        assert!((back.0 - 1250.0).abs() < 0.01 && (back.1 - 260.0).abs() < 0.01);
        assert_eq!(minimap_to_world(rect, 999.0, 300.0), None);
        assert_eq!(
            minimap_to_world(rect, 1200.0, 500.0),
            None,
            "past the bottom edge"
        );
    }

    #[test]
    fn ground_lines_project_and_are_cut_behind_the_camera() {
        let mut camera = at_zoom(0.5);
        camera.z = 30.0;
        let view = View::new(camera, W, H);
        let (ahead, behind) = ((2.0, 25.0), (2.0, 80.0));
        assert!(
            view.project(behind.0, 0.0, behind.1).is_none(),
            "really behind"
        );

        let (a, b) = view
            .project_ground_line(ahead, behind)
            .expect("part of it is visible");
        assert_eq!(
            a,
            view.project(ahead.0, 0.0, ahead.1).unwrap(),
            "the visible end stays put"
        );
        assert!(b.1 > a.1, "it runs towards the viewer");
        assert!(
            view.project_ground_line((0.0, 80.0), (5.0, 90.0)).is_none(),
            "all behind"
        );

        let whole = view.project_ground_line((0.0, 20.0), (5.0, 22.0)).unwrap();
        assert_eq!(whole.0, view.project(0.0, 0.0, 20.0).unwrap());
    }

    #[test]
    fn segments_clip_to_a_rectangle() {
        let area = (0.0, 0.0, 20.0, 10.0);
        assert_eq!(
            clip_to_rect((-5.0, 5.0), (25.0, 5.0), area),
            Some(((0.0, 5.0), (20.0, 5.0)))
        );
        assert_eq!(clip_to_rect((5.0, 12.0), (15.0, 12.0), area), None);
    }

    #[test]
    fn segments_clip_to_the_square() {
        let square = (0.0, 0.0, 10.0);
        let inside = ((1.0, 1.0), (9.0, 2.0));
        assert_eq!(clip_to_square(inside.0, inside.1, square), Some(inside));
        assert_eq!(
            clip_to_square((-5.0, 5.0), (15.0, 5.0), square),
            Some(((0.0, 5.0), (10.0, 5.0)))
        );
        assert_eq!(clip_to_square((-5.0, 20.0), (15.0, 20.0), square), None);
        assert_eq!(
            clip_to_square((5.0, -5.0), (5.0, 5.0), square),
            Some(((5.0, 0.0), (5.0, 5.0)))
        );
    }

    #[test]
    fn the_bottom_position_draws_without_gaps_at_the_screen_edges() {
        for zoom in [0.5, 1.0] {
            let mut out = Vec::new();
            draw(&mut out, terrain(0), None, at_zoom(zoom), W, H);
            assert!(!out.is_empty());
            let lowest = out.iter().map(|v| v.y).fold(f32::MIN, f32::max);
            assert!(
                lowest >= H,
                "terrain reaches the bottom of the screen at {zoom}: {lowest}"
            );
        }
    }

    #[test]
    fn map_corners_are_north_south_east_west_on_screen() {
        // A small diamond around the target: the real south corner is behind this camera.
        let camera = Camera::centered();
        let (middle, reach) = (LAST / 2.0, 20.0);
        let north = screen(camera, middle - reach, middle - reach, 0.0);
        let south = screen(camera, middle + reach, middle + reach, 0.0);
        let east = screen(camera, middle + reach, middle - reach, 0.0);
        let west = screen(camera, middle - reach, middle + reach, 0.0);
        assert!(
            north.1 < east.1 && east.1 < south.1,
            "north above, south below"
        );
        assert!((north.0 - W / 2.0).abs() < 0.5 && (south.0 - W / 2.0).abs() < 0.5);
        assert!(
            west.0 < W / 2.0 && W / 2.0 < east.0,
            "west left, east right"
        );
        assert!((east.1 - west.1).abs() < 0.5, "east and west are level");
    }

    #[test]
    fn the_camera_target_lands_at_screen_center() {
        let camera = Camera::centered();
        let (x, y) = View::new(camera, W, H)
            .project(camera.x, 0.0, camera.z)
            .unwrap();
        assert!((x - W / 2.0).abs() < 0.01 && (y - H / 2.0).abs() < 0.01);
    }

    #[test]
    fn perspective_makes_nearer_ground_look_bigger() {
        let camera = Camera::centered();
        let middle = LAST / 2.0;
        let span = |offset: f32| {
            let a = screen(camera, middle + offset, middle + offset, 0.0);
            let b = screen(camera, middle + offset + 1.0, middle + offset - 1.0, 0.0);
            (b.0 - a.0).abs()
        };
        assert!(span(10.0) > span(-10.0), "south of the target is nearer");
    }

    #[test]
    fn ground_stands_above_sand_and_is_darker() {
        let camera = Camera::centered();
        let middle = LAST / 2.0;
        assert!(
            screen(camera, middle, middle, GROUND_HEIGHT).1 < screen(camera, middle, middle, 0.0).1
        );
        assert!(GROUND.iter().sum::<f32>() < SAND.iter().sum::<f32>());
    }

    #[test]
    fn every_map_has_sand_and_ground_and_each_map_differs() {
        let counts: Vec<Vec<bool>> = (0..crate::game::MAPS.len())
            .map(|map| {
                let terrain = terrain(map);
                let ground = terrain.is_ground.iter().filter(|g| **g).count();
                let total = terrain.is_ground.len();
                assert!(
                    ground > total / 8 && ground < total * 2 / 3,
                    "map {map}: {ground} of {total}"
                );
                terrain.is_ground.clone()
            })
            .collect();
        for (i, a) in counts.iter().enumerate() {
            for b in &counts[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    fn map_on(planet: Planet) -> usize {
        MAP_PLANETS.iter().position(|p| *p == planet).unwrap()
    }

    #[test]
    fn map_names_start_with_their_planets_letter() {
        for (name, planet) in crate::game::MAPS.iter().zip(MAP_PLANETS) {
            assert!(
                name.starts_with(&format!("{planet:?} ")),
                "{name} on {planet:?}"
            );
        }
    }

    #[test]
    fn every_planet_has_a_map_and_looks_different() {
        let planets = [
            Planet::A,
            Planet::C,
            Planet::D,
            Planet::G,
            Planet::S,
            Planet::T,
        ];
        for (i, a) in planets.iter().enumerate() {
            assert!(MAP_PLANETS.contains(a));
            assert_ne!(
                a.lower_color(),
                a.upper_color(),
                "{a:?} needs two visible heights"
            );
            for b in &planets[i + 1..] {
                assert_ne!(a.lower_color(), b.lower_color(), "{a:?} and {b:?}");
            }
        }
    }

    #[test]
    fn planets_have_the_requested_colors() {
        let blue_ish = |c: [f32; 3]| c[2] > c[0] && c[2] > c[1];
        let green_ish = |c: [f32; 3]| c[1] > c[0] && c[1] > c[2];
        let red_ish = |c: [f32; 3]| c[0] > c[1] && c[0] > c[2];
        let bright = |c: [f32; 3]| c.iter().sum::<f32>() > 2.0;
        assert_eq!(Planet::A.lower_color(), SAND);
        assert!(blue_ish(Planet::C.lower_color()), "water");
        assert!(green_ish(Planet::C.upper_color()), "grass");
        assert!(
            blue_ish(Planet::D.lower_color()) && bright(Planet::D.lower_color()),
            "ice"
        );
        assert!(bright(Planet::D.upper_color()), "snow");
        assert!(red_ish(Planet::G.lower_color()), "lava");
        assert_eq!(
            Planet::G.upper_color(),
            Planet::A.upper_color(),
            "the same upper terrain"
        );
    }

    #[test]
    fn terrain_colors_follow_the_planet() {
        let water = terrain(map_on(Planet::C));
        assert_eq!(water.lower_color(), Planet::C.lower_color());
        assert_eq!(water.upper_color(), Planet::C.upper_color());
    }

    #[test]
    fn on_water_and_lava_every_start_area_is_land_and_ruins_stay_on_land() {
        for planet in [Planet::C, Planet::G] {
            let terrain = terrain(map_on(planet));
            let colors = [crate::game::TeamColor::Blue; 8];
            for count in 2..=colors.len() {
                for unit in crate::units::spawn(terrain, &colors[..count]) {
                    assert!(
                        (unit.y - GROUND_HEIGHT).abs() < 0.001,
                        "{planet:?} with {count}: {unit:?}"
                    );
                }
            }
            for piece in terrain.ruins() {
                let height = terrain.height_at(piece.x, piece.z);
                assert!(
                    (height - GROUND_HEIGHT).abs() < 0.001,
                    "{planet:?}: {piece:?}"
                );
            }
        }
    }

    #[test]
    fn every_map_has_destroyed_buildings_and_each_map_has_its_own() {
        let all: Vec<&[RuinPiece]> = (0..crate::game::MAPS.len())
            .map(|map| terrain(map).ruins())
            .collect();
        for (map, ruins) in all.iter().enumerate() {
            assert!(
                ruins.len() >= 15,
                "map {map} has only {} pieces",
                ruins.len()
            );
        }
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn ruins_stay_on_the_map_and_stand_on_its_terrain() {
        for map in 0..crate::game::MAPS.len() {
            let terrain = terrain(map);
            for piece in terrain.ruins() {
                let (u, v) = world_to_grid(piece.x, piece.z);
                assert!(
                    (0.0..=LAST).contains(&u) && (0.0..=LAST).contains(&v),
                    "{piece:?}"
                );
                assert!(piece.y == 0.0 || piece.y <= GROUND_HEIGHT, "{piece:?}");
                assert!(piece.height > 0.0 && piece.half_x > 0.0 && piece.half_z > 0.0);
            }
        }
    }

    #[test]
    fn ruins_are_repeatable_because_a_map_must_look_the_same_every_time() {
        let a = Terrain::generate(2);
        let b = Terrain::generate(2);
        assert_eq!(a.ruins(), b.ruins());
    }

    #[test]
    fn height_follows_the_terrain() {
        let terrain = terrain(0);
        for (u, v) in [(10, 10), (64, 64), (100, 30)] {
            let (x, z) = grid_to_world(u as f32, v as f32);
            let expected = if terrain.is_ground(u, v) {
                GROUND_HEIGHT
            } else {
                0.0
            };
            assert!((terrain.height_at(x, z) - expected).abs() < 1e-4);
        }
    }

    #[test]
    fn edges_push_their_way_and_corners_push_diagonally() {
        let d = std::f32::consts::FRAC_1_SQRT_2;
        let (r, b) = (W - 1.0, H - 1.0);
        let cases = [
            ((W / 2.0, H / 2.0), (0.0, 0.0)),
            ((0.0, H / 2.0), (-1.0, 0.0)),
            ((r, H / 2.0), (1.0, 0.0)),
            ((W / 2.0, 0.0), (0.0, -1.0)),
            ((W / 2.0, b), (0.0, 1.0)),
            ((0.0, 0.0), (-d, -d)),
            ((r, 0.0), (d, -d)),
            ((0.0, b), (-d, d)),
            ((r, b), (d, d)),
        ];
        for (mouse, expected) in cases {
            let got = edge_direction(mouse, W, H);
            assert!(
                (got.0 - expected.0).abs() < 1e-5 && (got.1 - expected.1).abs() < 1e-5,
                "{mouse:?} gave {got:?}"
            );
        }
        assert_eq!(
            edge_direction((30.0, 30.0), W, H),
            (0.0, 0.0),
            "near is not at the edge"
        );
    }

    #[test]
    fn diagonal_scrolling_is_no_faster_than_straight() {
        let d = std::f32::consts::FRAC_1_SQRT_2;
        let (mut straight, mut diagonal) = (Camera::centered(), Camera::centered());
        straight.scroll((1.0, 0.0), 5, 0.1);
        diagonal.scroll((d, d), 5, 0.1);
        let moved = |c: Camera| (c.x - Camera::centered().x).hypot(c.z - Camera::centered().z);
        assert!((moved(straight) - moved(diagonal)).abs() < 1e-3);
    }

    #[test]
    fn the_camera_stays_on_the_diamond_and_slides_along_its_edges() {
        let d = std::f32::consts::FRAC_1_SQRT_2;
        for direction in [
            (1.0, 0.0),
            (-1.0, 0.0),
            (0.0, 1.0),
            (0.0, -1.0),
            (d, d),
            (-d, -d),
            (d, -d),
            (-d, d),
        ] {
            let mut camera = Camera::centered();
            for _ in 0..2000 {
                camera.scroll(direction, 10, 0.05);
                let (u, v) = world_to_grid(camera.x, camera.z);
                assert!((-0.01..=LAST + 0.01).contains(&u) && (-0.01..=LAST + 0.01).contains(&v));
            }
        }
        // Pushing straight east ends at the east corner, not stuck on the first edge touched.
        let mut camera = Camera::centered();
        for _ in 0..2000 {
            camera.scroll((1.0, 0.0), 10, 0.05);
        }
        let (u, v) = world_to_grid(camera.x, camera.z);
        assert!(
            (u - LAST).abs() < 0.01 && v.abs() < 0.01,
            "ended at {u}, {v}"
        );
    }

    #[test]
    fn draw_only_emits_what_is_on_screen_and_follows_the_camera() {
        let terrain = terrain(0);
        let mut centered = Vec::new();
        draw(&mut centered, terrain, None, Camera::centered(), W, H);
        assert!(!centered.is_empty());
        assert_eq!(centered.len() % 6, 0);
        let all_quads = MAP_CELLS * MAP_CELLS * 6;
        assert!(
            centered.len() < all_quads,
            "culling keeps it below the whole map"
        );

        let mut moved = Vec::new();
        let mut camera = Camera::centered();
        camera.scroll((1.0, 0.0), 5, 0.5);
        draw(&mut moved, terrain, None, camera, W, H);
        assert_ne!(centered, moved);
    }

    #[test]
    fn draw_order_is_far_to_near() {
        let terrain = terrain(0);
        let mut out = Vec::new();
        draw(&mut out, terrain, None, Camera::centered(), W, H);
        // The first quad drawn is the farthest visible, so it sits higher on screen.
        let (first, last) = (out[0], out[out.len() - 1]);
        assert!(first.y < last.y);
    }
}
