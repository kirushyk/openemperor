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
use crate::platform::{Material, WorldVertex};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::OnceLock;

/// Grid cells along each side of a map of size S.
pub const MAP_CELLS: usize = 128;
/// Along each side of the biggest map. Every map's grid is kept to this stride, so a grid point
/// is found the same way on all of them.
const MOST_CELLS: usize = 256;
pub const GRID_POINTS: usize = (MOST_CELLS + 1) * (MOST_CELLS + 1);

/// How big a map is. Its features stay the same size: there is just more of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapSize {
    S,
    /// 1.5 times as long and wide as S.
    M,
    /// Twice as long and wide as S, four times its area.
    L,
}

impl MapSize {
    /// How many of something a map of this size has, for how many a map of size S has.
    fn spread(self, on_small: usize) -> usize {
        (on_small as f32 * self.scale() * self.scale()).round() as usize
    }

    pub fn cells(self) -> usize {
        match self {
            MapSize::S => MAP_CELLS,
            MapSize::M => MAP_CELLS * 3 / 2,
            MapSize::L => MOST_CELLS,
        }
    }

    /// How many times as long and wide as size S.
    pub fn scale(self) -> f32 {
        self.cells() as f32 / MAP_CELLS as f32
    }

    /// How far south of the north corner the middle of the map is.
    pub fn middle_z(self) -> f32 {
        self.cells() as f32 * CELL
    }
}

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
    Planet::A,
    Planet::C,
    Planet::D,
    Planet::G,
    Planet::S,
    Planet::T,
    Planet::A,
    Planet::C,
    Planet::D,
    Planet::G,
    Planet::S,
    Planet::T,
];

/// How big each map in `game::MAPS` is. Maps added later go at the end, so that those there
/// already stay as they are.
const fn map_size(map: usize) -> MapSize {
    match map {
        0..=9 => MapSize::S,
        10..=15 => MapSize::M,
        _ => MapSize::L,
    }
}

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
/// Nothing is farther than this from the camera.
const FAR: f32 = 500.0;

/// Pixels from the screen edge that start scrolling, on a 1080-pixel-tall screen.
const EDGE_MARGIN: f32 = 4.0;
/// World units per second, per point of the Scroll Rate option.
const SCROLL_PER_RATE: f32 = 15.0;

/// World z of the map's middle; its x is 0.
#[cfg(test)]
pub const MAP_MIDDLE_Z: f32 = MAP_CELLS as f32 * CELL;

/// How many destroyed buildings each map tries to place.
const RUIN_COUNT: usize = 24;
const MOUNTAIN_COUNT: usize = 9;
const PIT_COUNT: usize = 8;
/// Past this much rise or fall from the land, nothing can walk.
const STEEPEST_WALK: f32 = 0.25;

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
    size: MapSize,
    is_ground: Vec<bool>,
    /// How far mountains rise from the land, or pits sink into it, at each grid point.
    relief: Vec<f32>,
    /// How far the lower terrain lies under its noise's threshold, at each grid point.
    deep: Vec<f32>,
    colors: Vec<[f32; 3]>,
    ruins: Vec<RuinPiece>,
    /// Which way the land faces at each grid point, for the backend to light it by.
    normals: Vec<[f32; 3]>,
    /// Where lava erupts now and then.
    vents: Vec<(f32, f32)>,
    /// Which grid points a route may go through, worked out the first time it is needed.
    walkable: OnceLock<Vec<bool>>,
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
    v * (MOST_CELLS + 1) + u
}

impl Terrain {
    fn generate(map: usize) -> Self {
        let size = map_size(map);
        let mut is_ground = vec![false; GRID_POINTS];
        let mut colors = vec![[0.0; 3]; GRID_POINTS];
        let mut deep = vec![0.0; GRID_POINTS];
        let seed = 101 + map as u32 * 13;
        let planet = MAP_PLANETS[map];
        let threshold = if planet.lower_is_solid() {
            GROUND_THRESHOLD
        } else {
            LAND_THRESHOLD
        };
        for v in 0..=size.cells() {
            for u in 0..=size.cells() {
                let (fu, fv) = (u as f32 / MAP_CELLS as f32, v as f32 / MAP_CELLS as f32);
                let height = fractal_noise([fu * 2.5, fv * 2.5, 0.5], seed);
                let (x, z) = grid_to_world(u as f32, v as f32);
                let ground = height > threshold
                    || (!planet.lower_is_solid() && crate::units::needs_land(x, z, size));
                let grain = fractal_noise([fu * 30.0, fv * 30.0, 2.5], seed + 500) - 0.5;
                let base = if ground {
                    planet.upper_color()
                } else {
                    planet.lower_color()
                };
                let index = grid_index(u, v);
                is_ground[index] = ground;
                deep[index] = (threshold - height).max(0.0);
                colors[index] = base.map(|c| c * (1.0 + grain * 2.0 * GRAIN));
            }
        }
        let mut terrain = Terrain {
            planet,
            size,
            is_ground,
            relief: vec![0.0; GRID_POINTS],
            deep,
            colors,
            ruins: Vec::new(),
            normals: Vec::new(),
            vents: Vec::new(),
            walkable: OnceLock::new(),
        };
        let mut random = (seed + 1).wrapping_mul(0x9E37_79B1) | 1;
        terrain.raise_relief(seed, &mut random);
        let mut pieces = Vec::new();
        // Inset from the map's edges, so that no ruin hangs over them.
        let (inset, span) = (8.0, size.cells() as f32 - 16.0);
        // Land is scarcer where the lower terrain cannot hold anything, so look harder there.
        let ruin_count = size.spread(RUIN_COUNT);
        let tries = if planet.lower_is_solid() {
            ruin_count
        } else {
            ruin_count * 8
        };
        let mut placed = 0;
        for _ in 0..tries {
            let u = inset + next_random(&mut random) * span;
            let v = inset + next_random(&mut random) * span;
            let (x, z) = grid_to_world(u, v);
            if placed < ruin_count
                && crate::units::keeps_clear_for_ruin(x, z, size)
                && terrain.can_hold_ruin(x, z)
            {
                add_ruin(&mut pieces, &terrain, x, z, &mut random);
                placed += 1;
            }
        }
        terrain.ruins = pieces;
        terrain.normals = terrain.find_normals();
        terrain.vents = terrain.find_vents(&mut random);
        terrain
    }

    /// Spots well out in the lava, apart from each other, if this map has any.
    fn find_vents(&self, random: &mut u32) -> Vec<(f32, f32)> {
        const VENTS: usize = 10;
        const APART: f32 = 9.0;
        /// How far under the noise's threshold it must be, to be well away from the shore.
        const OUT_IN_IT: f32 = 0.02;
        let mut vents: Vec<(f32, f32)> = Vec::new();
        if self.material() != Material::Lava {
            return vents;
        }
        let count = self.size.spread(VENTS);
        for _ in 0..count * 100 {
            let (u, v) = (
                (next_random(random) * self.last()) as usize,
                (next_random(random) * self.last()) as usize,
            );
            let (x, z) = grid_to_world(u as f32, v as f32);
            let is_free = vents.iter().all(|o| (o.0 - x).hypot(o.1 - z) >= APART);
            let is_out_in_it = !self.is_ground(u, v) && self.deep[grid_index(u, v)] > OUT_IN_IT;
            if vents.len() < count && is_free && is_out_in_it {
                vents.push((x, z));
            }
        }
        vents
    }

    /// Which way the land faces at each grid point.
    fn find_normals(&self) -> Vec<[f32; 3]> {
        let step = CELL * std::f32::consts::SQRT_2;
        let cells = self.size.cells();
        let height = |u: usize, v: usize| self.vertex_height(u.min(cells), v.min(cells));
        let across = std::f32::consts::FRAC_1_SQRT_2;
        let mut normals = vec![[0.0, 1.0, 0.0]; GRID_POINTS];
        for v in 0..=cells {
            for u in 0..=cells {
                let along_u = (height(u + 1, v) - height(u.saturating_sub(1), v)) / (2.0 * step);
                let along_v = (height(u, v + 1) - height(u, v.saturating_sub(1))) / (2.0 * step);
                let slope_x = (along_u - along_v) * across;
                let slope_z = (along_u + along_v) * across;
                let length = (slope_x * slope_x + slope_z * slope_z + 1.0).sqrt();
                normals[grid_index(u, v)] = [-slope_x / length, 1.0 / length, -slope_z / length];
            }
        }
        normals
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
        for (count, is_mountain) in [
            (self.size.spread(MOUNTAIN_COUNT), true),
            (self.size.spread(PIT_COUNT), false),
        ] {
            let mut placed = 0;
            for _ in 0..tries * count / 8 {
                if placed == count {
                    break;
                }
                let (u, v) = (
                    8.0 + next_random(random) * (self.last() - 16.0),
                    8.0 + next_random(random) * (self.last() - 16.0),
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
    }

    /// Whether a mountain or pit could go here: clear of starts and of other relief, with a
    /// way around it, and on land throughout where the lower terrain cannot be stood on.
    fn has_room_for_relief(&self, x: f32, z: f32, radius: f32) -> bool {
        const AROUND: f32 = 2.0;
        (0..16).all(|i| {
            let (sin, cos) = (i as f32 / 16.0 * std::f32::consts::TAU).sin_cos();
            [radius, radius + AROUND].iter().all(|reach| {
                let (px, pz) = (x + cos * reach, z + sin * reach);
                crate::units::keeps_clear(px, pz, self.size)
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
        for v in 0..=self.size.cells() {
            for u in 0..=self.size.cells() {
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
        (0.0..=self.last()).contains(&u) && (0.0..=self.last()).contains(&v)
    }

    /// Flat sand holding only the given ruins, for testing how units meet them.
    #[cfg(test)]
    pub fn open_with_ruins(ruins: Vec<RuinPiece>) -> Self {
        Terrain {
            planet: Planet::A,
            size: MapSize::S,
            is_ground: vec![false; GRID_POINTS],
            relief: vec![0.0; GRID_POINTS],
            deep: vec![0.0; GRID_POINTS],
            colors: vec![SAND; GRID_POINTS],
            ruins,
            normals: vec![[0.0, 1.0, 0.0]; GRID_POINTS],
            vents: Vec::new(),
            walkable: OnceLock::new(),
        }
    }

    /// Flat upper ground with nothing on it, where buildings may go up.
    #[cfg(test)]
    pub fn open_upland() -> Self {
        Terrain {
            is_ground: vec![true; GRID_POINTS],
            ..Terrain::open_with_ruins(Vec::new())
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
        grid_to_world(u * self.last(), v * self.last())
    }

    pub fn size(&self) -> MapSize {
        self.size
    }

    /// How far along each grid axis the map goes.
    fn last(&self) -> f32 {
        self.size.cells() as f32
    }

    pub fn ruins(&self) -> &[RuinPiece] {
        &self.ruins
    }

    pub fn is_ground(&self, u: usize, v: usize) -> bool {
        self.is_ground[grid_index(u, v)]
    }

    /// What the ground shader makes of this map.
    pub fn material(&self) -> Material {
        match self.planet {
            Planet::A => Material::Sand,
            Planet::C => Material::Water,
            Planet::G => Material::Lava,
            Planet::D => Material::Ice,
            Planet::S => Material::Stone,
            Planet::T => Material::Moss,
        }
    }

    /// Where lava erupts now and then, if this map has any.
    pub fn vents(&self) -> &[(f32, f32)] {
        &self.vents
    }

    /// Whether this grid point is on a mountain or in a pit that nothing can walk on.
    pub fn is_relief(&self, u: usize, v: usize) -> bool {
        self.relief[grid_index(u, v)].abs() >= STEEPEST_WALK
    }

    /// The color of a grid point, with its slope lit.
    pub fn color_at(&self, u: usize, v: usize) -> [f32; 3] {
        self.colors[grid_index(u, v)]
    }

    /// Which grid points a route may go through: what can be walked on, and clear of ruins by
    /// enough to get past them.
    fn route_grid(&self) -> &[bool] {
        self.walkable.get_or_init(|| {
            /// How far from a ruin a route keeps.
            const CLEAR_OF_RUINS: f32 = 0.7;
            let mut grid: Vec<bool> = (0..GRID_POINTS)
                .map(|index| {
                    let (u, v) = (index % (MOST_CELLS + 1), index / (MOST_CELLS + 1));
                    let (x, z) = grid_to_world(u as f32, v as f32);
                    self.can_walk(x, z)
                })
                .collect();
            for piece in &self.ruins {
                let reach = piece.half_x.max(piece.half_z) + CLEAR_OF_RUINS;
                self.for_points_near(piece.x, piece.z, reach, |index, x, z| {
                    let dx = (x - piece.x).abs() - piece.half_x;
                    let dz = (z - piece.z).abs() - piece.half_z;
                    if dx.max(0.0).hypot(dz.max(0.0)) < CLEAR_OF_RUINS {
                        grid[index] = false;
                    }
                });
            }
            grid
        })
    }

    /// Calls back with each grid point on the map within a square around a place.
    fn for_points_near(&self, x: f32, z: f32, reach: f32, mut visit: impl FnMut(usize, f32, f32)) {
        let (u, v) = world_to_grid(x, z);
        // A step along one grid axis covers 0.7 of a world unit, so this reaches far enough.
        let span = reach * 1.5 + 1.0;
        let last = self.size.cells() as i32;
        let range = |center: f32| {
            ((center - span).floor() as i32).max(0)..=((center + span).ceil() as i32).min(last)
        };
        for point_v in range(v) {
            for point_u in range(u) {
                let (px, pz) = grid_to_world(point_u as f32, point_v as f32);
                visit(grid_index(point_u as usize, point_v as usize), px, pz);
            }
        }
    }

    /// Whether someone could walk straight from one point to the other, past ruins too. What
    /// lies right at either end is no matter: it may well stand beside a wall.
    pub fn line_is_clear(&self, from: (f32, f32), to: (f32, f32)) -> bool {
        Self::line_is_clear_on(self.route_grid(), from, to)
    }

    fn line_is_clear_on(grid: &[bool], from: (f32, f32), to: (f32, f32)) -> bool {
        const STEP: f32 = 0.2;
        const ENDS: f32 = 0.75;
        let length = (to.0 - from.0).hypot(to.1 - from.1);
        let steps = (length / STEP).ceil().max(1.0) as usize;
        (1..=steps).all(|i| {
            let t = i as f32 / steps as f32;
            let along = t * length;
            if along < ENDS || length - along < ENDS {
                return true;
            }
            let (u, v) = world_to_grid(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
            let (u, v) = (u.round(), v.round());
            u >= 0.0
                && v >= 0.0
                && u <= MOST_CELLS as f32
                && v <= MOST_CELLS as f32
                && grid[grid_index(u as usize, v as usize)]
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
        self.next_on_grid(self.route_grid(), from, to)
    }

    /// As `next_on_route`, keeping clear of these as well, each given as (x, z, how far).
    pub fn next_on_route_around(
        &self,
        from: (f32, f32),
        to: (f32, f32),
        obstacles: &[(f32, f32, f32)],
    ) -> Option<(f32, f32)> {
        let mut grid = self.route_grid().to_vec();
        for &(x, z, reach) in obstacles {
            self.for_points_near(x, z, reach, |index, px, pz| {
                if (px - x).hypot(pz - z) < reach {
                    grid[index] = false;
                }
            });
        }
        self.next_on_grid(&grid, from, to)
    }

    fn next_on_grid(
        &self,
        grid_walkable: &[bool],
        from: (f32, f32),
        to: (f32, f32),
    ) -> Option<(f32, f32)> {
        const STRAIGHT: u32 = 10;
        const DIAGONAL: u32 = 14;
        const NOWHERE: usize = usize::MAX;
        let walkable = |index: usize| grid_walkable[index];
        // The walkable grid point nearest a world point.
        let snap = |(x, z): (f32, f32)| -> Option<usize> {
            let (u, v) = world_to_grid(x, z);
            let (u, v) = (u.round() as i32, v.round() as i32);
            let last = self.size.cells() as i32;
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
                (index % (MOST_CELLS + 1)) as i32,
                (index / (MOST_CELLS + 1)) as i32,
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
                let last = self.size.cells() as i32;
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
            .find(|&spot| Self::line_is_clear_on(grid_walkable, from, spot))
            .or_else(|| route.iter().rev().nth(1).map(|&index| point(index)))
    }

    /// How far relief lifts or sinks a point, following its slopes.
    fn relief_at(&self, x: f32, z: f32) -> f32 {
        self.blend_at(x, z, |terrain, u, v| terrain.relief[grid_index(u, v)])
    }

    fn blend_at(&self, x: f32, z: f32, value: impl Fn(&Self, usize, usize) -> f32) -> f32 {
        let (u, v) = world_to_grid(x, z);
        let last = self.last();
        let (u, v) = (u.clamp(0.0, last), v.clamp(0.0, last));
        let (u0, v0) = (u.floor().min(last - 1.0), v.floor().min(last - 1.0));
        let (tu, tv) = (u - u0, v - v0);
        let at = |du: usize, dv: usize| value(self, u0 as usize + du, v0 as usize + dv);
        let near = at(0, 0) * (1.0 - tu) + at(1, 0) * tu;
        let far = at(0, 1) * (1.0 - tu) + at(1, 1) * tu;
        near * (1.0 - tv) + far * tv
    }

    /// The terrain's height under a world point, following its slopes.
    /// Whether a point is on the upper terrain: not on the sand, nor the lower ground that stands
    /// for it on other planets, nor in water or lava.
    pub fn on_upper_ground(&self, x: f32, z: f32) -> bool {
        if !self.on_map(x, z) {
            return false;
        }
        let (u, v) = world_to_grid(x, z);
        let last = self.size.cells();
        let (u, v) = (u.round() as usize, v.round() as usize);
        self.is_ground(u.min(last), v.min(last))
    }

    /// The lowest ground anyone can stand on: the sand, or the land above water and lava.
    pub fn floor(&self) -> f32 {
        if self.planet.lower_is_solid() {
            0.0
        } else {
            GROUND_HEIGHT
        }
    }

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

/// What turns a distance `d` from the camera into a depth from 0 to 1, as a + b / d.
fn depth_terms() -> (f32, f32) {
    (FAR / (FAR - NEAR), -NEAR * FAR / (FAR - NEAR))
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
    /// How far along each grid axis the map it is kept on goes.
    last: f32,
}

impl Camera {
    /// Looking at the middle of a map of size S from the top position.
    pub fn centered() -> Self {
        Self::centered_on(MapSize::S)
    }

    /// Looking at the middle of a map of this size from the top position.
    pub fn centered_on(size: MapSize) -> Self {
        let last = size.cells() as f32;
        let (x, z) = grid_to_world(last / 2.0, last / 2.0);
        Camera {
            x,
            z,
            zoom: 0.0,
            zoom_target: 0.0,
            last,
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
        (self.x, self.z) = grid_to_world(u.clamp(0.0, self.last), v.clamp(0.0, self.last));
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
        let last = self.last;
        let blocked = |position: f32, step: f32| {
            (position >= last && step > 0.0) || (position <= 0.0 && step < 0.0)
        };
        let slide = |other: f32, step: f32| {
            let middle = last / 2.0;
            let end = if other < middle {
                0.0
            } else if other > middle {
                last
            } else if step > 0.0 {
                0.0
            } else {
                last
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
        (self.x, self.z) = grid_to_world(u.clamp(0.0, self.last), v.clamp(0.0, self.last));
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

    /// What the backend places the world with, by columns. It lands on the same pixels as
    /// `project`, with depth from 0 at the near limit to 1 at the far one.
    pub fn matrix(&self) -> [f32; 16] {
        let (sx, sy) = (self.focal / self.center.0, self.focal / self.center.1);
        let [px, py, pz] = self.position;
        let (a, b) = depth_terms();
        let [f0, f1] = self.forward;
        let [u0, u1] = self.up;
        let along = f0 * py + f1 * pz;
        [
            sx,
            0.0,
            0.0,
            0.0,
            0.0,
            sy * u0,
            a * f0,
            f0,
            0.0,
            sy * u1,
            a * f1,
            f1,
            -sx * px,
            -sy * (u0 * py + u1 * pz),
            -a * along + b,
            -along,
        ]
    }

    /// `matrix` with everything as it looks in a mirror lying at height 0.
    pub fn mirror_matrix(&self) -> [f32; 16] {
        let mut matrix = self.matrix();
        for value in &mut matrix[4..8] {
            *value = -*value;
        }
        matrix
    }

    /// The depth the backend gives a world point, or `None` behind the camera.
    pub fn project_deep(&self, x: f32, y: f32, z: f32) -> Option<(f32, f32, f32)> {
        let (side, up, depth) = self.relative(x, y, z);
        let (a, b) = depth_terms();
        (depth >= NEAR).then(|| {
            let (px, py) = self.to_screen(side, up, depth);
            (px, py, a + b / depth)
        })
    }

    /// The part of the sand plane the screen shows, as (least x, least z, most x, most z), or
    /// `None` if the view reaches past the horizon.
    pub fn ground_seen(&self, width: f32, height: f32) -> Option<(f32, f32, f32, f32)> {
        let corners = [(0.0, 0.0), (width, 0.0), (width, height), (0.0, height)];
        let mut seen = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (px, py) in corners {
            let (x, z) = self.ground_at(px, py)?;
            seen = (seen.0.min(x), seen.1.min(z), seen.2.max(x), seen.3.max(z));
        }
        Some(seen)
    }

    /// The world as the sun sees it, by columns: over what the camera sees, in steps of the
    /// shadow map's pixels so that shadows do not crawl as the camera moves.
    pub fn sun_matrix(&self, focus: (f32, f32), width: f32, height: f32) -> [f32; 16] {
        /// The pixels across the backend's shadow map.
        const SHADOW_MAP: f32 = 4096.0;
        const MOST_REACH: f32 = 90.0;
        const ROOM: f32 = 8.0;
        /// How far along the sun's rays it takes in.
        const THROUGH: f32 = 90.0;
        let reach = self.ground_seen(width, height).map_or(MOST_REACH, |seen| {
            let far_x = (seen.0 - focus.0).abs().max((seen.2 - focus.0).abs());
            let far_z = (seen.1 - focus.1).abs().max((seen.3 - focus.1).abs());
            (far_x.hypot(far_z) + ROOM).min(MOST_REACH)
        });
        // In whole steps, so that it does not change size with every small move.
        let reach = (reach / 4.0).ceil() * 4.0;
        let cross = |a: [f32; 3], b: [f32; 3]| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let unit = |a: [f32; 3]| {
            let length = dot(a, a).sqrt();
            a.map(|c| c / length)
        };
        let sun = unit(SUN);
        let right = unit(cross([0.0, 0.0, 1.0], sun));
        let up = cross(sun, right);
        let texel = reach * 2.0 / SHADOW_MAP;
        let focus = [focus.0, 0.0, focus.1];
        let snap = |value: f32| (value / texel).round() * texel;
        let (x, y) = (snap(dot(focus, right)), snap(dot(focus, up)));
        let along = dot(focus, sun);
        let depth = -0.5 / THROUGH;
        [
            right[0] / reach,
            up[0] / reach,
            sun[0] * depth,
            0.0,
            right[1] / reach,
            up[1] / reach,
            sun[1] * depth,
            0.0,
            right[2] / reach,
            up[2] / reach,
            sun[2] * depth,
            0.0,
            -x / reach,
            -y / reach,
            0.5 - along * depth,
            1.0,
        ]
    }

    pub fn position(&self) -> [f32; 3] {
        self.position
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

    /// How many pixels a ball of this radius spans from its middle on the screen, if any of it
    /// is on the screen.
    pub fn ball_on_screen(&self, (x, y, z): (f32, f32, f32), radius: f32) -> Option<f32> {
        let (side, up, depth) = self.relative(x, y, z);
        if depth + radius < NEAR {
            return None;
        }
        let depth = depth.max(NEAR);
        let (px, py) = self.to_screen(side, up, depth);
        let pixels = self.focal * radius / depth;
        let (width, height) = (self.center.0 * 2.0, self.center.1 * 2.0);
        let on = px + pixels >= 0.0
            && px - pixels <= width
            && py + pixels >= 0.0
            && py - pixels <= height;
        on.then_some(pixels)
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
pub fn world_to_minimap(
    (x, y, size): (f32, f32, f32),
    middle_z: f32,
    world_x: f32,
    world_z: f32,
) -> (f32, f32) {
    (
        x + size / 2.0 + world_x / middle_z * size / 2.0,
        y + world_z / middle_z * size / 2.0,
    )
}

/// The world point under a minimap position, or `None` outside the minimap's square.
pub fn minimap_to_world(
    (x, y, size): (f32, f32, f32),
    middle_z: f32,
    px: f32,
    py: f32,
) -> Option<(f32, f32)> {
    if px < x || px >= x + size || py < y || py >= y + size {
        return None;
    }
    Some((
        (px - (x + size / 2.0)) / (size / 2.0) * middle_z,
        (py - y) / (size / 2.0) * middle_z,
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

/// Appends the terrain near what the camera sees, with room around it for what casts a shadow
/// into view.
pub fn draw(
    out: &mut Vec<WorldVertex>,
    terrain: &Terrain,
    fog: Option<&Fog>,
    camera: Camera,
    width: f32,
    height: f32,
) {
    /// Under a liquid's surface along the shore, however shallow the bottom is there.
    const SHORE: f32 = 0.12;
    /// How far down a liquid goes for how far under the threshold its noise is.
    const DEPTH: f32 = 6.0;
    /// What the land is made of right at the water, so its color does not run blue.
    const BEACH: [f32; 3] = [0.74, 0.68, 0.48];
    /// The same at the lava, burnt black.
    const BASALT: [f32; 3] = [0.10, 0.08, 0.07];
    const ROOM: f32 = 8.0;
    let shore = match terrain.material() {
        Material::Water => Some(BEACH),
        Material::Lava => Some(BASALT),
        Material::Sand | Material::Ice | Material::Stone | Material::Moss => None,
    };
    let view = View::new(camera, width, height);
    let point = |u: usize, v: usize| {
        let (x, z) = grid_to_world(u as f32, v as f32);
        let index = grid_index(u, v);
        let height = terrain.vertex_height(u, v);
        let ([r, g, b], level) = match shore {
            Some(color) if !terrain.is_ground[index] => {
                (color, -(SHORE + terrain.deep[index] * DEPTH))
            }
            // A pit in the land stays dry, however far down it goes.
            Some(_) => (terrain.colors[index], height.max(0.001)),
            None => (terrain.colors[index], height),
        };
        WorldVertex {
            x,
            y: height,
            z,
            normal: terrain.normals[index],
            r,
            g,
            b,
            seen: fog.map_or(1.0, |fog| fog.at_grid(u, v).shade()),
            level,
            stain: [0.0; 3],
        }
    };
    let area = view.ground_seen(width, height);
    let is_near = |u: usize, v: usize| {
        let (x, z) = grid_to_world(u as f32 + 0.5, v as f32 + 0.5);
        area.is_none_or(|(min_x, min_z, max_x, max_z)| {
            x >= min_x - ROOM && x <= max_x + ROOM && z >= min_z - ROOM && z <= max_z + ROOM
        })
    };
    let cells = terrain.size.cells();
    for u in 0..cells {
        for v in (0..cells).filter(|&v| is_near(u, v)) {
            let (a, b) = (point(u, v), point(u + 1, v));
            let (c, d) = (point(u + 1, v + 1), point(u, v + 1));
            out.extend([a, b, c, a, c, d]);
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
            world_to_minimap(rect, MAP_MIDDLE_Z, x, z)
        };
        assert_eq!(corner(0.0, 0.0), (1200.0, 100.0), "north is top middle");
        assert_eq!(
            corner(LAST, LAST),
            (1200.0, 500.0),
            "south is bottom middle"
        );
        assert_eq!(corner(LAST, 0.0), (1400.0, 300.0), "east is right middle");
        assert_eq!(corner(0.0, LAST), (1000.0, 300.0), "west is left middle");

        let (x, z) = minimap_to_world(rect, MAP_MIDDLE_Z, 1250.0, 260.0).unwrap();
        let back = world_to_minimap(rect, MAP_MIDDLE_Z, x, z);
        assert!((back.0 - 1250.0).abs() < 0.01 && (back.1 - 260.0).abs() < 0.01);
        assert_eq!(minimap_to_world(rect, MAP_MIDDLE_Z, 999.0, 300.0), None);
        assert_eq!(
            minimap_to_world(rect, MAP_MIDDLE_Z, 1200.0, 500.0),
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
            let view = View::new(at_zoom(zoom), W, H);
            for x in [0.0, W / 2.0, W] {
                let (ground_x, ground_z) = view.ground_at(x, H).unwrap();
                assert!(
                    out.iter()
                        .any(|v| (v.x - ground_x).abs() < 1.0 && (v.z - ground_z).abs() < 1.0),
                    "terrain reaches the bottom of the screen at {zoom}"
                );
            }
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
                let side = terrain.size().cells() + 1;
                let total = side * side;
                assert!(
                    ground > total / 8 && ground < total * 3 / 4,
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
                for unit in crate::units::spawn_bases(terrain, &colors[..count]) {
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
            let last = terrain.size().cells() as f32;
            for piece in terrain.ruins() {
                let (u, v) = world_to_grid(piece.x, piece.z);
                assert!(
                    (0.0..=last).contains(&u) && (0.0..=last).contains(&v),
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
            for v in 0..=terrain.size().cells() {
                for u in 0..=terrain.size().cells() {
                    let (x, z) = grid_to_world(u as f32, v as f32);
                    if terrain.is_relief(u, v) {
                        assert!(!terrain.can_walk(x, z), "map {map} at {u},{v}");
                        blocked += 1;
                    }
                    if terrain.relief[grid_index(u, v)].abs() > 0.0 {
                        assert!(
                            crate::units::keeps_clear(x, z, terrain.size()),
                            "map {map} near a start"
                        );
                    }
                }
            }
            assert!(blocked > 0);
            assert!(
                blocked < GRID_POINTS / 5,
                "map {map} is not all rock: {blocked}"
            );
        }
    }

    #[test]
    fn only_lava_has_vents_and_they_lie_out_in_it_apart_from_each_other() {
        let lava = terrain(map_on(Planet::G));
        assert!(lava.vents().len() >= 3, "{}", lava.vents().len());
        for (i, &(x, z)) in lava.vents().iter().enumerate() {
            assert!(lava.on_map(x, z) && !lava.can_walk(x, z), "{x},{z}");
            for &(ox, oz) in &lava.vents()[i + 1..] {
                assert!((x - ox).hypot(z - oz) >= 9.0);
            }
        }
        for planet in [Planet::A, Planet::C, Planet::D, Planet::S, Planet::T] {
            assert!(terrain(map_on(planet)).vents().is_empty(), "{planet:?}");
        }
    }

    /// The first map of this size.
    fn map_of(size: MapSize) -> usize {
        (0..crate::game::MAPS.len())
            .find(|&map| map_size(map) == size)
            .unwrap()
    }

    #[test]
    fn maps_come_in_three_sizes_on_every_planet_with_their_things_spread_over_them() {
        assert_eq!(MapSize::S.cells(), MAP_CELLS);
        assert_eq!(MapSize::M.cells() * 2, MAP_CELLS * 3, "1.5 times as long");
        assert_eq!(MapSize::L.cells(), MAP_CELLS * 2, "twice as long");
        for size in [MapSize::S, MapSize::M, MapSize::L] {
            for planet in [
                Planet::A,
                Planet::C,
                Planet::D,
                Planet::G,
                Planet::S,
                Planet::T,
            ] {
                let there = (0..crate::game::MAPS.len())
                    .any(|map| map_size(map) == size && MAP_PLANETS[map] == planet);
                assert!(there, "{planet:?} at {size:?}");
            }
        }
        let (s, l) = (terrain(map_of(MapSize::S)), terrain(map_of(MapSize::L)));
        assert_eq!((s.size(), l.size()), (MapSize::S, MapSize::L));
        let (far_x, far_z) = grid_to_world(250.0, 250.0);
        assert!(l.on_map(far_x, far_z) && !s.on_map(far_x, far_z));
        assert!(!l.on_map(far_x, far_z + 20.0));
        assert!(
            l.ruins().len() > s.ruins().len() * 2,
            "about four times as many"
        );
        let mut random = 7;
        let beyond = (0..100)
            .map(|_| l.random_point(&mut random))
            .filter(|&(x, z)| !s.on_map(x, z))
            .count();
        assert!(beyond > 40, "{beyond}");
    }

    #[test]
    fn maps_of_one_size_are_not_copies_of_another() {
        let first = |size: MapSize| terrain(map_of(size));
        let ground = |terrain: &Terrain| {
            (0..100)
                .map(|i| terrain.is_ground(i + 5, i + 5))
                .collect::<Vec<_>>()
        };
        assert_ne!(ground(first(MapSize::S)), ground(first(MapSize::M)));
        assert_ne!(ground(first(MapSize::S)), ground(first(MapSize::L)));
    }

    #[test]
    fn the_camera_is_kept_on_a_map_of_its_size_and_starts_in_its_middle() {
        let mut camera = Camera::centered_on(MapSize::L);
        assert_eq!(camera.z, MapSize::L.middle_z());
        assert_eq!(MapSize::L.middle_z(), MAP_MIDDLE_Z * 2.0);
        camera.look_at(0.0, 1000.0);
        let (u, v) = world_to_grid(camera.x, camera.z);
        assert!((u - 256.0).abs() < 0.01 && (v - 256.0).abs() < 0.01);
        let mut small = Camera::centered();
        small.look_at(0.0, 1000.0);
        assert!(small.z < camera.z);
    }

    #[test]
    fn the_minimap_shows_all_of_a_map_of_any_size() {
        let rect = (0.0, 0.0, 200.0);
        for size in [MapSize::S, MapSize::M, MapSize::L] {
            let (x, z) = grid_to_world(size.cells() as f32, size.cells() as f32);
            let (px, py) = world_to_minimap(rect, size.middle_z(), x, z);
            assert!(
                (px - 100.0).abs() < 0.01 && (py - 200.0).abs() < 0.01,
                "{size:?}"
            );
        }
    }

    #[test]
    fn a_route_can_be_found_right_across_a_big_map() {
        let terrain = terrain(map_of(MapSize::L));
        let from = terrain.nearest_walkable(grid_to_world(20.0, 20.0)).unwrap();
        let to = terrain
            .nearest_walkable(grid_to_world(236.0, 236.0))
            .unwrap();
        assert!(terrain.next_on_route(from, to).is_some());
    }

    #[test]
    fn each_map_gets_the_material_of_its_planet() {
        assert_eq!(terrain(map_on(Planet::A)).material(), Material::Sand);
        assert_eq!(terrain(map_on(Planet::C)).material(), Material::Water);
        assert_eq!(terrain(map_on(Planet::G)).material(), Material::Lava);
        assert_eq!(terrain(map_on(Planet::D)).material(), Material::Ice);
        assert_eq!(terrain(map_on(Planet::S)).material(), Material::Stone);
        assert_eq!(terrain(map_on(Planet::T)).material(), Material::Moss);
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
        let (water, land): (Vec<WorldVertex>, Vec<WorldVertex>) =
            out.iter().partition(|v| v.level < 0.0);
        assert!(!water.is_empty() && !land.is_empty());
        for vertex in water {
            let (u, v) = world_to_grid(vertex.x, vertex.z);
            assert!(!terrain.is_ground(u.round() as usize, v.round() as usize));
        }
    }

    /// Where a matrix puts a world point: on the screen, in pixels, and its depth.
    fn through(matrix: [f32; 16], (x, y, z): (f32, f32, f32)) -> (f32, f32, f32) {
        let row = |i: usize| matrix[i] * x + matrix[4 + i] * y + matrix[8 + i] * z + matrix[12 + i];
        let w = row(3);
        (
            (row(0) / w + 1.0) / 2.0 * W,
            (1.0 - row(1) / w) / 2.0 * H,
            row(2) / w,
        )
    }

    fn near(a: f32, b: f32, by: f32) -> bool {
        (a - b).abs() < by
    }

    #[test]
    fn the_matrix_the_backend_gets_lands_on_the_same_pixels_and_depth_as_the_game_works_with() {
        for zoom in [0.0, 0.5, 1.0] {
            let view = View::new(at_zoom(zoom), W, H);
            let (x, z) = grid_to_world(64.0, 64.0);
            for point in [
                (x, 0.0, z),
                (x + 3.0, 2.0, z - 4.0),
                (x - 6.0, 0.7, z + 1.0),
            ] {
                let (px, py, depth) = through(view.matrix(), point);
                let (sx, sy, sd) = view.project_deep(point.0, point.1, point.2).unwrap();
                assert!(
                    near(px, sx, 0.05) && near(py, sy, 0.05),
                    "{zoom}: {px},{py} {sx},{sy}"
                );
                assert!(near(depth, sd, 1e-4), "{depth} {sd}");
                assert!(depth > 0.0 && depth < 1.0);
            }
        }
    }

    #[test]
    fn what_is_nearer_the_camera_has_less_depth_in_the_matrix() {
        let view = View::new(at_zoom(0.5), W, H);
        let (x, z) = grid_to_world(64.0, 64.0);
        let (_, _, far) = through(view.matrix(), (x, 0.0, z - 5.0));
        let (_, _, near_it) = through(view.matrix(), (x, 0.0, z + 5.0));
        assert!(near_it < far);
    }

    #[test]
    fn a_mirror_shows_what_is_above_it_lower_on_the_screen_and_what_is_at_it_in_place() {
        let view = View::new(Camera::centered(), W, H);
        let (x, z) = grid_to_world(64.0, 64.0);
        let above = through(view.matrix(), (x, 2.0, z));
        let reflected = through(view.mirror_matrix(), (x, 2.0, z));
        assert!(reflected.1 > above.1);
        let at_it = through(view.matrix(), (x, 0.0, z));
        let at_it_reflected = through(view.mirror_matrix(), (x, 0.0, z));
        assert!(near(at_it.1, at_it_reflected.1, 0.01));
    }

    #[test]
    fn the_sun_sees_the_camera_area_with_what_is_nearer_it_at_less_depth() {
        let camera = at_zoom(0.0);
        let view = View::new(camera, W, H);
        let sun_view = view.sun_matrix((camera.x, camera.z), W, H);
        let at = |y: f32| {
            let (x, z) = (camera.x, camera.z);
            let row = |i: usize| {
                sun_view[i] * x + sun_view[4 + i] * y + sun_view[8 + i] * z + sun_view[12 + i]
            };
            (row(0), row(1), row(2))
        };
        let (ground_x, ground_y, ground) = at(0.0);
        assert!(
            ground_x.abs() < 0.01 && ground_y.abs() < 0.01,
            "in the middle of its map"
        );
        let (_, _, high) = at(5.0);
        assert!(high < ground, "nearer the sun");
        assert!(ground > 0.0 && ground < 1.0);
    }

    #[test]
    fn normals_are_unit_length_and_lean_away_from_rising_land() {
        let terrain = Terrain::open_with_mountain(0.0, 40.0, 5.0);
        let normals = terrain.find_normals();
        for normal in &normals {
            let length = normal.iter().map(|c| c * c).sum::<f32>().sqrt();
            assert!((length - 1.0).abs() < 1e-3);
        }
        let flat = Terrain::open_with_ruins(Vec::new()).find_normals();
        assert_eq!(flat[grid_index(5, 5)], [0.0, 1.0, 0.0]);
    }

    #[test]
    fn a_route_goes_around_a_ruin_and_around_what_it_is_told_to_keep_clear_of() {
        let wall = RuinPiece {
            x: 0.0,
            y: 0.0,
            z: 40.0,
            half_x: 6.0,
            half_z: 0.1,
            height: 1.0,
            shade: 1.0,
        };
        let terrain = Terrain::open_with_ruins(vec![wall]);
        let (from, to) = ((0.0, 35.0), (0.0, 45.0));
        assert!(!terrain.line_is_clear(from, to), "the wall is in the way");
        let next = terrain.next_on_route(from, to).unwrap();
        assert!(next.0.abs() > 3.0, "off to one end of it: {next:?}");

        let open = Terrain::open_with_ruins(Vec::new());
        assert!(open.line_is_clear(from, to));
        let around = open
            .next_on_route_around(from, to, &[(0.0, 40.0, 2.0)])
            .unwrap();
        assert!(open.line_is_clear(from, around));
        assert!(around != to && around.0.abs() > 0.5, "{around:?}");
        assert_eq!(
            open.next_on_route_around(from, to, &[]).map(|_| ()),
            Some(())
        );
    }

    #[test]
    fn a_wall_right_at_either_end_of_a_line_does_not_count_as_in_the_way() {
        let wall = RuinPiece {
            x: 0.0,
            y: 0.0,
            z: 40.0,
            half_x: 6.0,
            half_z: 0.1,
            height: 1.0,
            shade: 1.0,
        };
        let terrain = Terrain::open_with_ruins(vec![wall]);
        // Beside the wall, going away from it.
        assert!(terrain.line_is_clear((0.0, 39.5), (0.0, 30.0)));
        assert!(terrain.line_is_clear((0.0, 30.0), (0.0, 39.5)));
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
        // A backend with no depth buffer has only this order to go by.
        let (first, last) = (out[0], out[out.len() - 1]);
        assert!(first.z < last.z);
    }
}
