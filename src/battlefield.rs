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
use crate::platform::{GroundVertex, Material, Vertex};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::OnceLock;

pub const MAP_CELLS: usize = 128;
const GRID_LAST: f32 = MAP_CELLS as f32;
pub const GRID_POINTS: usize = (MAP_CELLS + 1) * (MAP_CELLS + 1);
/// World units per grid step along each world axis.
const CELL: f32 = 0.5;
/// How far ground rises above sand, in world units.
const GROUND_HEIGHT: f32 = 0.7;
/// The way to the sun, as a unit vector: high up on the screen's left and a little behind what
/// it lights, so that shadows fall to the right and towards the camera.
pub const SUN: [f32; 3] = [-0.50, 0.80, -0.33];
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
const MOUNTAIN_COUNT: usize = 9;
const PIT_COUNT: usize = 8;
/// Past this much rise or fall from the land, nothing can walk.
const STEEPEST_WALK: f32 = 0.25;
/// A cell with all corners this high is drawn in turn with what stands on the map.
const RAISED: f32 = 0.2;

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
    /// How far mountains rise from the land, or pits sink into it, at each grid point.
    relief: Vec<f32>,
    /// How far the lower terrain lies under its noise's threshold, at each grid point.
    deep: Vec<f32>,
    colors: Vec<[f32; 3]>,
    ruins: Vec<RuinPiece>,
    /// The cells that stick up out of the ground, nearest the far side first. Things can stand
    /// behind those, so they are drawn in turn with what stands on the map.
    raised_cells: Vec<RaisedCell>,
    /// Which grid points can be walked on, worked out the first time a route is looked for.
    walkable: OnceLock<Vec<bool>>,
}

/// A grid cell of a mountain, and how far along towards the camera it lies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaisedCell {
    pub u: usize,
    pub v: usize,
    pub z: f32,
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
        let mut deep = Vec::with_capacity(GRID_POINTS);
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
                deep.push((threshold - height).max(0.0));
                colors.push(base.map(|c| c * (1.0 + grain * 2.0 * GRAIN)));
            }
        }
        let mut terrain = Terrain {
            planet,
            is_ground,
            relief: vec![0.0; GRID_POINTS],
            deep,
            colors,
            ruins: Vec::new(),
            raised_cells: Vec::new(),
            walkable: OnceLock::new(),
        };
        let mut random = (seed + 1).wrapping_mul(0x9E37_79B1) | 1;
        terrain.raise_relief(seed, &mut random);
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
        terrain.light_slopes();
        terrain
    }

    /// Brightens the colors where the land turns to the sun and darkens them where it turns
    /// away, against how flat land looks.
    fn light_slopes(&mut self) {
        const DARKEST: f32 = 0.6;
        /// A low bank only dims, or the coarse grid would show as teeth along it.
        const DARKEST_BANK: f32 = 0.8;
        const BRIGHTEST: f32 = 1.2;
        const IN_SHADOW: f32 = 0.72;
        let step = CELL * std::f32::consts::SQRT_2;
        let height = |u: usize, v: usize| self.vertex_height(u.min(MAP_CELLS), v.min(MAP_CELLS));
        let mut lit = self.colors.clone();
        for v in 0..=MAP_CELLS {
            for u in 0..=MAP_CELLS {
                let along_u = (height(u + 1, v) - height(u.saturating_sub(1), v)) / (2.0 * step);
                let along_v = (height(u, v + 1) - height(u, v.saturating_sub(1))) / (2.0 * step);
                let across = std::f32::consts::FRAC_1_SQRT_2;
                let (slope_x, slope_z) =
                    ((along_u - along_v) * across, (along_u + along_v) * across);
                let length = (slope_x * slope_x + slope_z * slope_z + 1.0).sqrt();
                let facing = (-slope_x * SUN[0] + SUN[1] - slope_z * SUN[2]) / length;
                let darkest = if self.relief[grid_index(u, v)] == 0.0 {
                    DARKEST_BANK
                } else {
                    DARKEST
                };
                let mut light = (facing / SUN[1]).clamp(darkest, BRIGHTEST);
                if self.sun_is_blocked(u, v) {
                    light = light.min(1.0) * IN_SHADOW;
                }
                lit[grid_index(u, v)] = lit[grid_index(u, v)].map(|c| c * light);
            }
        }
        self.colors = lit;
    }

    /// Whether the land between this grid point and the sun stands in the way.
    fn sun_is_blocked(&self, u: usize, v: usize) -> bool {
        /// Far enough for the shadow of the tallest mountain.
        const REACH: f32 = 9.0;
        const STEP: f32 = 0.2;
        /// So that a slope does not shade itself, nor a low bank what is at its foot, which
        /// the grid is too coarse to show as anything but teeth.
        const LIFT: f32 = 0.8;
        let (x, z) = grid_to_world(u as f32, v as f32);
        let y = self.vertex_height(u, v) + LIFT;
        (1..=(REACH / STEP) as usize).any(|i| {
            let t = i as f32 * STEP;
            let (px, pz) = (x + SUN[0] * t, z + SUN[2] * t);
            self.on_map(px, pz) && self.height_at(px, pz) > y + SUN[1] * t
        })
    }

    fn vertex_height(&self, u: usize, v: usize) -> f32 {
        let base = if self.is_ground(u, v) {
            GROUND_HEIGHT
        } else {
            0.0
        };
        base + self.relief[grid_index(u, v)]
    }

    /// Puts up the mountains and digs the pits, well away from wherever a group can start.
    fn raise_relief(&mut self, seed: u32, random: &mut u32) {
        // Where a planet has little land, it takes longer to find room for them.
        let tries = if self.planet.lower_is_solid() {
            30
        } else {
            240
        };
        for (count, is_mountain) in [(MOUNTAIN_COUNT, true), (PIT_COUNT, false)] {
            let mut placed = 0;
            for _ in 0..tries * count / 8 {
                if placed == count {
                    break;
                }
                let (u, v) = (
                    8.0 + next_random(random) * (GRID_LAST - 16.0),
                    8.0 + next_random(random) * (GRID_LAST - 16.0),
                );
                let (x, z) = grid_to_world(u, v);
                let radius = if is_mountain {
                    2.6 + next_random(random) * 3.4
                } else {
                    1.8 + next_random(random) * 2.4
                };
                if self.has_room_for_relief(x, z, radius) {
                    let size = 0.7 + next_random(random) * 0.6;
                    self.add_relief(
                        (x, z),
                        radius,
                        size,
                        is_mountain,
                        seed + 900 + placed as u32,
                    );
                    placed += 1;
                }
            }
        }
        self.raised_cells = (0..MAP_CELLS)
            .flat_map(|u| (0..MAP_CELLS).map(move |v| (u, v)))
            .filter(|&(u, v)| self.is_raised_cell(u, v))
            .map(|(u, v)| RaisedCell {
                u,
                v,
                z: (u + v + 1) as f32 * CELL,
            })
            .collect();
    }

    /// Whether a mountain or pit could go here: clear of starts and of other relief, with a
    /// way around it, and on land throughout where the lower terrain cannot be stood on.
    fn has_room_for_relief(&self, x: f32, z: f32, radius: f32) -> bool {
        const AROUND: f32 = 2.0;
        (0..16).all(|i| {
            let (sin, cos) = (i as f32 / 16.0 * std::f32::consts::TAU).sin_cos();
            [radius, radius + AROUND].iter().all(|reach| {
                let (px, pz) = (x + cos * reach, z + sin * reach);
                crate::units::keeps_clear(px, pz)
                    && self.can_walk(px, pz)
                    && self.can_walk(x + cos * reach * 0.5, z + sin * reach * 0.5)
            })
        }) && self.can_walk(x, z)
    }

    /// A mountain with ridges down its sides, or a pit with steep walls and a dark floor.
    fn add_relief(
        &mut self,
        (x, z): (f32, f32),
        radius: f32,
        size: f32,
        is_mountain: bool,
        seed: u32,
    ) {
        const PEAK: f32 = 5.0;
        const PIT_DEPTH: f32 = 1.6;
        const ROCK: [f32; 3] = [0.56, 0.50, 0.45];
        const SNOW: [f32; 3] = [0.86, 0.87, 0.90];
        const DARK_ROCK: [f32; 3] = [0.38, 0.33, 0.30];
        const FLOOR: [f32; 3] = [0.05, 0.04, 0.04];
        let reach = radius + 1.0;
        for v in 0..=MAP_CELLS {
            for u in 0..=MAP_CELLS {
                let (px, pz) = grid_to_world(u as f32, v as f32);
                if (px - x).abs() > reach || (pz - z).abs() > reach {
                    continue;
                }
                let (fu, fv) = (u as f32 / MAP_CELLS as f32, v as f32 / MAP_CELLS as f32);
                let warp = fractal_noise([fu * 14.0, fv * 14.0, 4.5], seed) - 0.5;
                let distance = (px - x).hypot(pz - z) / radius + warp * 0.5;
                let inside = (1.0 - distance).clamp(0.0, 1.0);
                let index = grid_index(u, v);
                if inside <= 0.0 || !self.is_ground[index] && !self.planet.lower_is_solid() {
                    continue;
                }
                if is_mountain {
                    let noise = fractal_noise([fu * 26.0, fv * 26.0, 6.5], seed + 1);
                    let ridge = 1.0 - (noise * 2.0 - 1.0).abs();
                    let fine = fractal_noise([fu * 70.0, fv * 70.0, 8.5], seed + 2);
                    let rise = inside * inside.sqrt();
                    let share = (rise * (0.25 + 1.1 * ridge * ridge + 0.3 * fine)).min(1.0);
                    self.relief[index] = self.relief[index].max(share * PEAK * size);
                    let grain = self.colors[index][0] / self.base_of(index)[0].max(0.01);
                    // Bands of rock as it climbs, all of it rock past the very foot.
                    let band = fractal_noise([share * 9.0, fine * 2.0, 3.5], seed + 3);
                    let stone = mix(DARK_ROCK, ROCK, band * 1.5);
                    let rock = mix(self.base_of(index), stone, (share * 30.0).min(1.0));
                    let snowy = ((share - 0.78 + (fine - 0.5) * 0.25) * 9.0).clamp(0.0, 1.0);
                    let color = mix(rock, SNOW, snowy);
                    self.colors[index] = color.map(|c| c * grain);
                } else {
                    // Steep walls down to a floor that is flat, and dark for how little sun
                    // gets down there.
                    let wall = (inside * 3.0).min(1.0);
                    let fall = wall * wall * (3.0 - 2.0 * wall);
                    self.relief[index] = self.relief[index].min(-fall * PIT_DEPTH * size);
                    self.colors[index] = mix(self.colors[index], FLOOR, fall * 0.9);
                }
            }
        }
    }

    fn base_of(&self, index: usize) -> [f32; 3] {
        if self.is_ground[index] {
            self.planet.upper_color()
        } else {
            self.planet.lower_color()
        }
    }

    /// Nothing stands in water or lava, so there the whole site must be on land.
    fn can_hold_ruin(&self, x: f32, z: f32) -> bool {
        const SITE: f32 = 2.4;
        let site_is_flat = [
            (-SITE, -SITE),
            (SITE, -SITE),
            (SITE, SITE),
            (-SITE, SITE),
            (0.0, 0.0),
        ]
        .iter()
        .all(|(dx, dz)| self.relief_at(x + dx, z + dz) == 0.0);
        site_is_flat && self.planet.lower_is_solid()
            || site_is_flat
                && [
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
            && self.relief_at(x, z).abs() < STEEPEST_WALK
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
            relief: vec![0.0; GRID_POINTS],
            deep: vec![0.0; GRID_POINTS],
            colors: vec![SAND; GRID_POINTS],
            ruins,
            raised_cells: Vec::new(),
            walkable: OnceLock::new(),
        }
    }

    /// Flat sand with a mountain in the way, for testing how units get around it.
    #[cfg(test)]
    pub fn open_with_mountain(x: f32, z: f32, radius: f32) -> Self {
        let mut terrain = Self::open_with_ruins(Vec::new());
        terrain.add_relief((x, z), radius, 1.0, true, 7);
        terrain
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

    fn is_raised_cell(&self, u: usize, v: usize) -> bool {
        [(u, v), (u + 1, v), (u + 1, v + 1), (u, v + 1)]
            .iter()
            .all(|&(u, v)| self.relief[grid_index(u, v)] > RAISED)
    }

    /// What the ground shader makes of this map.
    pub fn material(&self) -> Material {
        match self.planet {
            Planet::A => Material::Sand,
            Planet::C => Material::Water,
            Planet::D | Planet::G | Planet::S | Planet::T => Material::Plain,
        }
    }

    pub fn raised_cells(&self) -> &[RaisedCell] {
        &self.raised_cells
    }

    /// Whether this grid point is on a mountain or in a pit that nothing can walk on.
    pub fn is_relief(&self, u: usize, v: usize) -> bool {
        self.relief[grid_index(u, v)].abs() >= STEEPEST_WALK
    }

    /// The color of a grid point, with its slope lit.
    pub fn color_at(&self, u: usize, v: usize) -> [f32; 3] {
        self.colors[grid_index(u, v)]
    }

    /// Whether someone could walk straight from one point to the other.
    pub fn line_is_clear(&self, from: (f32, f32), to: (f32, f32)) -> bool {
        const STEP: f32 = 0.2;
        let length = (to.0 - from.0).hypot(to.1 - from.1);
        let steps = (length / STEP).ceil().max(1.0) as usize;
        (1..=steps).all(|i| {
            let t = i as f32 / steps as f32;
            self.can_walk(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
        })
    }

    /// The nearest point to this one that can be walked on, if any is near enough to matter.
    pub fn nearest_walkable(&self, (x, z): (f32, f32)) -> Option<(f32, f32)> {
        const RINGS: usize = 40;
        const PER_RING: usize = 24;
        if self.can_walk(x, z) {
            return Some((x, z));
        }
        (1..=RINGS).find_map(|ring| {
            let reach = ring as f32 * CELL;
            (0..PER_RING).find_map(|i| {
                let (sin, cos) = (i as f32 / PER_RING as f32 * std::f32::consts::TAU).sin_cos();
                let spot = (x + cos * reach, z + sin * reach);
                self.can_walk(spot.0, spot.1).then_some(spot)
            })
        })
    }

    /// The first place to head for on the way around whatever lies between two points: as far
    /// along the shortest route as can be reached in a straight line. `None` if there is no
    /// way there at all.
    pub fn next_on_route(&self, from: (f32, f32), to: (f32, f32)) -> Option<(f32, f32)> {
        const STRAIGHT: u32 = 10;
        const DIAGONAL: u32 = 14;
        const NOWHERE: usize = usize::MAX;
        let grid_walkable = self.walkable.get_or_init(|| {
            (0..GRID_POINTS)
                .map(|index| {
                    let (u, v) = (index % (MAP_CELLS + 1), index / (MAP_CELLS + 1));
                    let (x, z) = grid_to_world(u as f32, v as f32);
                    self.can_walk(x, z)
                })
                .collect()
        });
        let walkable = |index: usize| grid_walkable[index];
        // The walkable grid point nearest a world point.
        let snap = |(x, z): (f32, f32)| -> Option<usize> {
            let (u, v) = world_to_grid(x, z);
            let (u, v) = (u.round() as i32, v.round() as i32);
            let last = MAP_CELLS as i32;
            (0..=4).find_map(|reach: i32| {
                (-reach..=reach)
                    .flat_map(|du| (-reach..=reach).map(move |dv| (u + du, v + dv)))
                    .filter(|&(u, v)| u >= 0 && v >= 0 && u <= last && v <= last)
                    .map(|(u, v)| grid_index(u as usize, v as usize))
                    .find(|&index| walkable(index))
            })
        };
        let (start, goal) = (snap(from)?, snap(to)?);
        let grid = |index: usize| {
            (
                (index % (MAP_CELLS + 1)) as i32,
                (index / (MAP_CELLS + 1)) as i32,
            )
        };
        let (goal_u, goal_v) = grid(goal);
        let estimate = |index: usize| {
            let (u, v) = grid(index);
            let (du, dv) = ((u - goal_u).unsigned_abs(), (v - goal_v).unsigned_abs());
            STRAIGHT * du.max(dv) + (DIAGONAL - STRAIGHT) * du.min(dv)
        };
        let mut cost = vec![u32::MAX; GRID_POINTS];
        let mut came_from = vec![NOWHERE; GRID_POINTS];
        let mut open = BinaryHeap::new();
        cost[start] = 0;
        open.push((Reverse(estimate(start)), start));
        while let Some((_, index)) = open.pop() {
            if index == goal {
                break;
            }
            let (u, v) = grid(index);
            for (du, dv) in [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                let (nu, nv) = (u + du, v + dv);
                let last = MAP_CELLS as i32;
                if nu < 0 || nv < 0 || nu > last || nv > last {
                    continue;
                }
                let next = grid_index(nu as usize, nv as usize);
                // No cutting a corner that cannot be walked on.
                let diagonal = du != 0 && dv != 0;
                if !walkable(next)
                    || diagonal
                        && (!walkable(grid_index(nu as usize, v as usize))
                            || !walkable(grid_index(u as usize, nv as usize)))
                {
                    continue;
                }
                let step = if diagonal { DIAGONAL } else { STRAIGHT };
                let reached = cost[index] + step;
                if reached < cost[next] {
                    cost[next] = reached;
                    came_from[next] = index;
                    open.push((Reverse(reached + estimate(next)), next));
                }
            }
        }
        if came_from[goal] == NOWHERE && goal != start {
            return None;
        }
        let mut route = vec![goal];
        while let Some(&last) = route.last() {
            match came_from[last] {
                NOWHERE => break,
                earlier => route.push(earlier),
            }
        }
        let point = |index: usize| {
            let (u, v) = grid(index);
            grid_to_world(u as f32, v as f32)
        };
        // The route runs from the goal back to here, so the first clear point is the farthest.
        route
            .iter()
            .map(|&index| point(index))
            .find(|&spot| self.line_is_clear(from, spot))
            .or_else(|| route.iter().rev().nth(1).map(|&index| point(index)))
    }

    /// How far relief lifts or sinks a point, following its slopes.
    fn relief_at(&self, x: f32, z: f32) -> f32 {
        self.blend_at(x, z, |terrain, u, v| terrain.relief[grid_index(u, v)])
    }

    fn blend_at(&self, x: f32, z: f32, value: impl Fn(&Self, usize, usize) -> f32) -> f32 {
        let (u, v) = world_to_grid(x, z);
        let (u, v) = (u.clamp(0.0, GRID_LAST), v.clamp(0.0, GRID_LAST));
        let (u0, v0) = (
            u.floor().min(GRID_LAST - 1.0),
            v.floor().min(GRID_LAST - 1.0),
        );
        let (tu, tv) = (u - u0, v - v0);
        let at = |du: usize, dv: usize| value(self, u0 as usize + du, v0 as usize + dv);
        let near = at(0, 0) * (1.0 - tu) + at(1, 0) * tu;
        let far = at(0, 1) * (1.0 - tu) + at(1, 1) * tu;
        near * (1.0 - tv) + far * tv
    }

    /// The terrain's height under a world point, following its slopes.
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        self.blend_at(x, z, Self::vertex_height)
    }
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
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
    /// Seeing everything as it looks in a mirror lying on the ground at height 0.
    mirrored: bool,
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
            mirrored: false,
        }
    }

    /// The same view of everything as it looks in a mirror lying at height 0.
    pub fn mirrored(mut self) -> Self {
        self.mirrored = true;
        self
    }

    pub fn is_mirrored(&self) -> bool {
        self.mirrored
    }

    pub fn position(&self) -> [f32; 3] {
        self.position
    }

    /// Where the camera is in the world, as (x, z).
    pub fn position_on_ground(&self) -> (f32, f32) {
        (self.position[0], self.position[2])
    }

    /// A world point seen from the camera: how far to the side, how far up, and how deep.
    fn relative(&self, x: f32, y: f32, z: f32) -> (f32, f32, f32) {
        let [px, py, pz] = self.position;
        let y = if self.mirrored { -y } else { y };
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
    out: &mut Vec<GroundVertex>,
    terrain: &Terrain,
    fog: Option<&Fog>,
    camera: Camera,
    width: f32,
    height: f32,
) {
    /// Under the water's surface along the shore, however shallow the bottom is there.
    const SHORE: f32 = 0.12;
    /// How far down the water goes for how far under the threshold its noise is.
    const DEPTH: f32 = 6.0;
    /// What the land is made of right at the water, so its color does not run blue.
    const BEACH: [f32; 3] = [0.74, 0.68, 0.48];
    let has_water = terrain.material() == Material::Water;
    let view = View::new(camera, width, height);
    let point = |u: usize, v: usize| {
        let (x, z) = grid_to_world(u as f32, v as f32);
        let index = grid_index(u, v);
        let is_water = has_water && !terrain.is_ground[index];
        let ([r, g, b], level) = if is_water {
            (BEACH, -(SHORE + terrain.deep[index] * DEPTH))
        } else {
            // A pit in the land stays dry, however far down it goes.
            let height = terrain.vertex_height(u, v);
            let level = if has_water { height.max(0.001) } else { height };
            (terrain.colors[index], level)
        };
        view.project(x, terrain.vertex_height(u, v), z)
            .map(|(px, py)| GroundVertex {
                x: px,
                y: py,
                r,
                g,
                b,
                seen: fog.map_or(1.0, |fog| fog.at_grid(u, v).shade()),
                world_x: x,
                level,
                world_z: z,
            })
    };
    for u in 0..MAP_CELLS {
        for v in 0..MAP_CELLS {
            if terrain.is_raised_cell(u, v) {
                continue;
            }
            let corners = [
                point(u, v),
                point(u + 1, v),
                point(u + 1, v + 1),
                point(u, v + 1),
            ];
            if let [Some(a), Some(b), Some(c), Some(d)] = corners
                && !all_off_screen(&[a, b, c, d].map(|p| (p.x, p.y)), width, height)
            {
                out.extend([a, b, c, a, c, d]);
            }
        }
    }
}

/// Appends one cell of the terrain, unless it is off the screen.
pub fn draw_cell(
    out: &mut Vec<Vertex>,
    terrain: &Terrain,
    fog: Option<&Fog>,
    view: &View,
    (u, v): (usize, usize),
    (width, height): (f32, f32),
) {
    let point = |u: usize, v: usize| {
        let (x, z) = grid_to_world(u as f32, v as f32);
        let shade = fog.map_or(1.0, |fog| fog.at_grid(u, v).shade());
        let [r, g, b] = terrain.colors[grid_index(u, v)].map(|channel| channel * shade);
        view.project(x, terrain.vertex_height(u, v), z)
            .map(|point| Vertex::new(point, [r, g, b, 1.0]))
    };
    let corners = [
        point(u, v),
        point(u + 1, v),
        point(u + 1, v + 1),
        point(u, v + 1),
    ];
    if let [Some(a), Some(b), Some(c), Some(d)] = corners
        && !all_off_screen(&[a, b, c, d].map(|p| (p.x, p.y)), width, height)
    {
        out.extend([a, b, c, a, c, d]);
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
    fn every_map_has_mountains_and_pits_that_cannot_be_walked_on_and_rise_and_sink_as_they_should()
    {
        for map in 0..crate::game::MAPS.len() {
            let terrain = terrain(map);
            let highest = terrain.relief.iter().copied().fold(0.0, f32::max);
            let deepest = terrain.relief.iter().copied().fold(0.0, f32::min);
            assert!(highest > 1.5, "map {map} has mountains: {highest}");
            assert!(deepest < -0.7, "map {map} has pits: {deepest}");
            let mut blocked = 0;
            for v in 0..=MAP_CELLS {
                for u in 0..=MAP_CELLS {
                    let (x, z) = grid_to_world(u as f32, v as f32);
                    if terrain.is_relief(u, v) {
                        assert!(!terrain.can_walk(x, z), "map {map} at {u},{v}");
                        blocked += 1;
                    }
                    if terrain.relief[grid_index(u, v)].abs() > 0.0 {
                        assert!(crate::units::keeps_clear(x, z), "map {map} near a start");
                    }
                }
            }
            assert!(blocked > 0);
            assert!(
                blocked < GRID_POINTS / 5,
                "map {map} is not all rock: {blocked}"
            );
            assert!(!terrain.raised_cells().is_empty());
        }
    }

    #[test]
    fn a_mountain_shades_the_ground_on_its_far_side_from_the_sun_only() {
        let mut terrain = Terrain::open_with_ruins(Vec::new());
        let (peak_u, peak_v) = (64, 64);
        terrain.relief[grid_index(peak_u, peak_v)] = 5.0;
        let (x, z) = grid_to_world(peak_u as f32, peak_v as f32);
        let at = |reach: f32| {
            let (u, v) = world_to_grid(x + SUN[0] * reach, z + SUN[2] * reach);
            terrain.sun_is_blocked(u.round() as usize, v.round() as usize)
        };
        assert!(at(-2.0), "away from the sun");
        assert!(!at(2.0), "on the side towards it");
        assert!(!at(-8.0), "out of its reach");
    }

    #[test]
    fn each_map_gets_the_material_of_its_planet() {
        assert_eq!(terrain(map_on(Planet::A)).material(), Material::Sand);
        assert_eq!(terrain(map_on(Planet::C)).material(), Material::Water);
        assert_eq!(terrain(map_on(Planet::D)).material(), Material::Plain);
    }

    #[test]
    fn water_lies_below_its_surface_and_the_land_above_it_for_the_ground_shader() {
        let map = map_on(Planet::C);
        let terrain = terrain(map);
        let mut out = Vec::new();
        for (x, z) in [(0.0, 64.0), (-20.0, 40.0), (20.0, 90.0)] {
            let mut camera = Camera::centered();
            camera.x = x;
            camera.z = z;
            draw(&mut out, terrain, None, camera, W, H);
        }
        assert!(!out.is_empty());
        let (water, land): (Vec<GroundVertex>, Vec<GroundVertex>) =
            out.iter().partition(|v| v.level < 0.0);
        assert!(!water.is_empty() && !land.is_empty());
        for vertex in water {
            let (u, v) = world_to_grid(vertex.world_x, vertex.world_z);
            assert!(!terrain.is_ground(u.round() as usize, v.round() as usize));
        }
    }

    #[test]
    fn a_mirrored_view_shows_what_is_above_it_lower_on_the_screen() {
        let view = View::new(Camera::centered(), W, H);
        let mirror = View::new(Camera::centered(), W, H).mirrored();
        let (x, z) = grid_to_world(64.0, 64.0);
        let above = view.project(x, 2.0, z).unwrap();
        let reflected = mirror.project(x, 2.0, z).unwrap();
        assert!(reflected.1 > above.1);
        assert_eq!(view.project(x, 0.0, z), mirror.project(x, 0.0, z));
    }

    #[test]
    fn a_route_goes_around_a_mountain_and_there_is_none_into_a_walled_off_place() {
        let (x, z) = (0.0, 40.0);
        let terrain = Terrain::open_with_mountain(x, z, 5.0);
        let (from, to) = ((x, z - 9.0), (x, z + 9.0));
        assert!(!terrain.line_is_clear(from, to));
        let next = terrain.next_on_route(from, to).expect("a way around");
        assert!(terrain.line_is_clear(from, next));
        assert!(
            next != to && next.0.abs() > 1.0,
            "off to one side: {next:?}"
        );

        let clear = terrain.nearest_walkable((x, z)).expect("land nearby");
        assert!(terrain.can_walk(clear.0, clear.1));
        assert_eq!(terrain.nearest_walkable(from), Some(from));
        let open = Terrain::open_with_ruins(Vec::new());
        assert_eq!(open.next_on_route(from, to).map(|_| ()), Some(()));
    }

    #[test]
    fn height_follows_the_terrain() {
        let terrain = terrain(0);
        for (u, v) in [(10, 10), (64, 64), (100, 30)] {
            let (x, z) = grid_to_world(u as f32, v as f32);
            let expected = terrain.vertex_height(u, v);
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
