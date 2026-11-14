//! The battlefield: one of the original's maps, and the camera over it, projected on the CPU.
//!
//! A map is the original's own: square cells `CELL_SIDE` across, `width` of them along world x
//! and `height` along world z, a height at each corner of a cell, one of the original's kinds of
//! ground in each, spice in some, and the places its sides start from. World x runs along the
//! columns, y up and z along the rows, in the original's own coordinates. Grid points (u, v) are
//! the corners of the cells: column u, row v.
//!
//! The camera looks at the map turned an eighth of a turn, as the original's does, so that it
//! shows as a diamond: its first corner, (0, 0), at the top.

use crate::fog::Fog;
use crate::platform::Material;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::f32::consts::FRAC_1_SQRT_2;
use std::sync::OnceLock;

/// The middle of the biggest map, which tests are mostly set on.
#[cfg(test)]
pub const MAP_MIDDLE: (f32, f32) = (
    MOST_CELLS as f32 * CELL_SIDE / 2.0,
    MOST_CELLS as f32 * CELL_SIDE / 2.0,
);

/// Along each side of the biggest map. Every map's grid is kept to this stride, so a grid point
/// is found the same way on all of them.
pub const MOST_CELLS: usize = 256;
pub const GRID_POINTS: usize = (MOST_CELLS + 1) * (MOST_CELLS + 1);

/// How long the side of a cell is, in world units: a coordinate of the original game, whose
/// cells are 32 across and whose models, speeds and sizes are all given in those coordinates.
pub const CELL_SIDE: f32 = 32.0;
/// The way to the sun, as a unit vector: high up on the screen's left and a little behind what
/// it lights, so that shadows fall to the right and towards the camera.
pub const SUN: [f32; 3] = [-0.5869, 0.80, 0.1202];
/// How a map is lit when it does not say: from `SUN`, by a cool sky and a warm sun.
pub const DAYLIGHT: Light = Light {
    sun: SUN,
    ambient: [0.42, 0.45, 0.52],
    sunlight: [0.62, 0.58, 0.50],
};
/// The sand's color, for the dust thrown up from it.
pub const SAND: [f32; 3] = [0.78, 0.64, 0.40];

/// Which ways on the ground the screen's right and its bottom are, as (x, z): the map's columns
/// run down to the right and its rows down to the left.
const SCREEN_RIGHT: (f32, f32) = (FRAC_1_SQRT_2, -FRAC_1_SQRT_2);
const SCREEN_DOWN: (f32, f32) = (FRAC_1_SQRT_2, FRAC_1_SQRT_2);

/// Zooming moves the camera between two fixed positions and narrows the field of view at
/// the same time, in one movement. Top: high and looking almost straight down.
const TOP_HEIGHT: f32 = 1358.0;
const TOP_PITCH_DEGREES: f32 = 75.0;
/// Bottom: low and looking almost forward, 30 degrees down from the ground.
const BOTTOM_HEIGHT: f32 = 362.0;
const BOTTOM_PITCH_DEGREES: f32 = 30.0;
/// How much the field of view narrows at the very end of the zoom.
const MAX_LENS: f32 = 2.0;
/// Share of the whole zoom per wheel notch.
const ZOOM_PER_NOTCH: f32 = 0.05;
/// How quickly the camera catches up with the wheel; higher is snappier.
const ZOOM_EASING: f32 = 10.0;
/// Vertical field of view, as the tangent of half its angle (about 40 degrees in total).
const HALF_FOV_TAN: f32 = 0.364;
const NEAR: f32 = 45.25;
/// Nothing is farther than this from the camera.
const FAR: f32 = 22630.0;

/// Pixels from the screen edge that start scrolling, on a 1080-pixel-tall screen.
const EDGE_MARGIN: f32 = 4.0;
/// World units per second, per point of the Scroll Rate option.
const SCROLL_PER_RATE: f32 = 678.8;

/// The original's kinds of ground, in the order its `Rules.txt` lists them, which is the order
/// its maps number them in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ground {
    Sand,
    /// Where buildings go up.
    Rock,
    Cliff,
    /// Rock that can be driven on but not built on.
    NbRock,
    /// Rock only infantry can cross.
    InfRock,
    DustBowl,
    MapEdge,
    Ramp,
}

impl Ground {
    fn of(number: u8) -> Self {
        match number {
            1 => Ground::Rock,
            2 => Ground::Cliff,
            3 => Ground::NbRock,
            4 => Ground::InfRock,
            5 => Ground::DustBowl,
            6 => Ground::MapEdge,
            7 => Ground::Ramp,
            _ => Ground::Sand,
        }
    }

    /// Whether it can be gone over this way: cliffs only in the air, the rock with its stairs
    /// on foot too.
    fn lets(self, going: Going) -> bool {
        match self {
            Ground::Cliff | Ground::MapEdge => going == Going::Flying,
            Ground::InfRock => going != Going::Driving,
            _ => true,
        }
    }
}

/// How someone gets about, which decides where they can go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Going {
    OnFoot,
    Driving,
    Flying,
}

/// The parts of a map's file the original keeps its map in, ahead of the map's shape: each a
/// tag, its length and then itself.
const SIZE: u32 = 0xa000_0002;
const HEIGHTS: u32 = 0xa000_0001;
const GROUNDS: u32 = 0xa000_0003;
const SPICE: u32 = 0xa000_0004;
const AREAS: u32 = 0xa000_0005;
/// How long an area's or a list's name is, and one of its points: a number, then x and z.
const AREA_NAME: usize = 20;
const AREA_POINT: usize = 20;
/// The area holding the places the sides start from.
const BASES: &[u8] = b"Base";
/// As much spice as a cell holds.
const FULL_SPICE: u8 = 255;
/// How wide the map's radar picture is, in its pixels, one to a cell.
const RADAR_SIDE: usize = 256;

pub struct Terrain {
    width: usize,
    height: usize,
    /// How high each grid point is, by `grid_index`.
    heights: Vec<f32>,
    /// The ground of each cell, row by row.
    grounds: Vec<Ground>,
    /// How much spice each cell has, row by row: none at 0, as much as a cell holds at 255.
    spice: Vec<u8>,
    /// Where each side may start, as world points.
    starts: Vec<(f32, f32)>,
    /// The color of each cell on the radar, row by row.
    radar: Vec<[f32; 3]>,
    light: Light,
    /// Which grid points a route may go through, each way of going, worked out the first time
    /// it is needed.
    walkable: [OnceLock<Vec<bool>>; 3],
}

/// How a map is lit: the way to its sun as a unit vector, the light everything gets from all
/// around, and the light a face turned to the sun gets on top of that.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    pub sun: [f32; 3],
    pub ambient: [f32; 3],
    pub sunlight: [f32; 3],
}

impl Light {
    /// A map's light as its `test.lit` has it: the way to the sun as x, y and z, with z the
    /// other way along from ours, then the ambient light and the sunlight, each as red, green
    /// and blue from 0 to 255. What comes after those lights something else.
    pub fn read(text: &str) -> Option<Self> {
        let mut lines = text.lines().map(|line| {
            line.split_whitespace()
                .map(str::parse::<f32>)
                .collect::<Result<Vec<_>, _>>()
                .ok()
                .filter(|numbers| numbers.len() == 3)
                .map(|numbers| [numbers[0], numbers[1], numbers[2]])
        });
        let [x, y, z] = lines.next()??;
        let length = (x * x + y * y + z * z).sqrt();
        let mut color = || lines.next().flatten().map(|rgb| rgb.map(|c| c / 255.0));
        (length > 0.0).then_some(Light {
            sun: [x / length, y / length, -z / length],
            ambient: color()?,
            sunlight: color()?,
        })
    }
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

pub fn grid_index(u: usize, v: usize) -> usize {
    v * (MOST_CELLS + 1) + u
}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

fn f64_at(data: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(data.get(at..at + 8)?.try_into().ok()?))
}

/// The parts of a map's file, by their tags.
fn parts(file: &[u8]) -> Option<Vec<(u32, &[u8])>> {
    let length = u32_at(file, 4)? as usize;
    let end = 8 + length;
    let mut at = 8;
    let mut parts = Vec::new();
    while at + 8 <= end {
        let (tag, size) = (u32_at(file, at)?, u32_at(file, at + 4)? as usize);
        parts.push((tag, file.get(at + 8..at + 8 + size)?));
        at += 8 + size;
    }
    Some(parts)
}

/// The points of the lists in the area of this name, one list after another. The areas are a
/// count of them, then each: its name, a count of its lists, and each list: its name, a count
/// of its points, and the points.
fn area_points(areas: &[u8], wanted: &[u8]) -> Option<Vec<(f32, f32)>> {
    let mut at = 4;
    for _ in 0..u32_at(areas, 0)? {
        let name = areas.get(at..at + AREA_NAME)?;
        let is_wanted = name.split(|&byte| byte == 0).next() == Some(wanted);
        let lists = u32_at(areas, at + AREA_NAME)?;
        at += AREA_NAME + 4;
        let mut points = Vec::new();
        for _ in 0..lists {
            let count = u32_at(areas, at + AREA_NAME)? as usize;
            at += AREA_NAME + 4;
            for point in 0..count {
                let start = at + point * AREA_POINT + 4;
                points.push((
                    f64_at(areas, start)? as f32,
                    f64_at(areas, start + 8)? as f32,
                ));
            }
            at += count * AREA_POINT;
        }
        if is_wanted {
            return Some(points);
        }
    }
    None
}

/// How many places to start from a map's `test.xbf` has.
pub fn starts_in(file: &[u8]) -> usize {
    parts(file)
        .and_then(|parts| parts.into_iter().find(|&(tag, _)| tag == AREAS))
        .and_then(|(_, areas)| area_points(areas, BASES))
        .map_or(0, |points| points.len())
}

impl Terrain {
    /// One of the original's maps: its `test.xbf`, and its radar picture if it has one.
    pub fn read(file: &[u8], radar: Option<&[u8]>) -> Option<Self> {
        let parts = parts(file)?;
        let part = |tag: u32| parts.iter().find(|&&(t, _)| t == tag).map(|&(_, d)| d);
        let size = part(SIZE)?;
        let (width, height) = (u32_at(size, 0)? as usize, u32_at(size, 4)? as usize);
        if width == 0 || height == 0 || width > MOST_CELLS || height > MOST_CELLS {
            return None;
        }
        let cells = width * height;
        let corners = part(HEIGHTS)?;
        let mut heights = vec![0.0; GRID_POINTS];
        for v in 0..=height {
            for u in 0..=width {
                let at = (v * (width + 1) + u) * 4;
                heights[grid_index(u, v)] =
                    f32::from_le_bytes(corners.get(at..at + 4)?.try_into().ok()?);
            }
        }
        let grounds = part(GROUNDS)?
            .get(..cells)?
            .iter()
            .map(|&n| Ground::of(n))
            .collect();
        let spice = part(SPICE)?.get(..cells)?.to_vec();
        let starts = part(AREAS)
            .and_then(|areas| area_points(areas, BASES))
            .unwrap_or_default();
        let radar = (0..cells)
            .map(|cell| {
                let (u, v) = (cell % width, cell / width);
                let at = (v * RADAR_SIDE + u) * 2;
                radar
                    .and_then(|picture| picture.get(at..at + 2))
                    .map_or(SAND, |pixel| {
                        rgb565(u16::from_le_bytes([pixel[0], pixel[1]]))
                    })
            })
            .collect();
        Some(Terrain {
            width,
            height,
            heights,
            grounds,
            spice,
            starts,
            radar,
            light: DAYLIGHT,
            walkable: Default::default(),
        })
    }

    /// The same, lit this way.
    pub fn lit(self, light: Light) -> Self {
        Terrain { light, ..self }
    }

    /// Flat sand full of spice, this many cells each way, with a side starting on rock in each
    /// corner, for when a map cannot be read and for tests.
    pub fn blank(width: usize, height: usize) -> Self {
        /// How many cells each way the rock under a start reaches.
        const BASE: usize = 32;
        let (right, bottom) = (width as f32 * CELL_SIDE, height as f32 * CELL_SIDE);
        let inset = BASE as f32 / 2.0 * CELL_SIDE;
        let sand = Terrain {
            width,
            height,
            heights: vec![0.0; GRID_POINTS],
            grounds: vec![Ground::Sand; width * height],
            spice: vec![FULL_SPICE; width * height],
            starts: vec![
                (inset, inset),
                (right - inset, bottom - inset),
                (right - inset, inset),
                (inset, bottom - inset),
            ],
            radar: vec![SAND; width * height],
            light: DAYLIGHT,
            walkable: Default::default(),
        };
        let (far_u, far_v) = (width.saturating_sub(BASE), height.saturating_sub(BASE));
        [(0, 0), (far_u, far_v), (far_u, 0), (0, far_v)]
            .into_iter()
            .fold(sand, |terrain, corner| {
                terrain.with_ground(Ground::Rock, corner, (BASE, BASE))
            })
    }

    /// Flat sand as big as the biggest map, for tests.
    #[cfg(test)]
    pub fn open() -> Self {
        Self::blank(MOST_CELLS, MOST_CELLS)
    }

    /// Flat rock as big as the biggest map, where buildings may go up, for tests.
    #[cfg(test)]
    pub fn open_rock() -> Self {
        Self::open().with_ground(Ground::Rock, (0, 0), (MOST_CELLS, MOST_CELLS))
    }

    /// Flat sand with a round cliff of this radius in the way, for tests of how units get
    /// around it.
    #[cfg(test)]
    pub fn open_with_cliff(x: f32, z: f32, radius: f32) -> Self {
        let mut terrain = Self::open();
        for cell in 0..terrain.grounds.len() {
            let (u, v) = (cell % terrain.width, cell / terrain.width);
            let (cx, cz) = grid_to_world(u as f32 + 0.5, v as f32 + 0.5);
            if (cx - x).hypot(cz - z) < radius {
                terrain.grounds[cell] = Ground::Cliff;
            }
        }
        terrain
    }

    /// The same with the ground of the cells in a rectangle of them, from (u, v) on, changed,
    /// and spice left only on sand.
    pub fn with_ground(
        mut self,
        ground: Ground,
        (u, v): (usize, usize),
        (across, down): (usize, usize),
    ) -> Self {
        for row in v..(v + down).min(self.height) {
            for column in u..(u + across).min(self.width) {
                self.grounds[row * self.width + column] = ground;
                if ground != Ground::Sand {
                    self.spice[row * self.width + column] = 0;
                }
            }
        }
        self.walkable = Default::default();
        self
    }

    /// The same with spice in the cells of a rectangle of them.
    #[cfg(test)]
    pub fn with_spice(mut self, (u, v): (usize, usize), (across, down): (usize, usize)) -> Self {
        for row in v..(v + down).min(self.height) {
            for column in u..(u + across).min(self.width) {
                self.spice[row * self.width + column] = FULL_SPICE;
            }
        }
        self
    }

    /// How many cells along x and along z.
    pub fn cells(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// How far the map reaches along x and along z, in world units.
    pub fn extent(&self) -> (f32, f32) {
        (
            self.width as f32 * CELL_SIDE,
            self.height as f32 * CELL_SIDE,
        )
    }

    /// Where the sides may start, as world points.
    pub fn starts(&self) -> &[(f32, f32)] {
        &self.starts
    }

    /// What the ground shader makes of this map: only its pictures.
    pub fn material(&self) -> Material {
        Material::Pictured
    }

    /// The cell a world point is in, if it is on the map.
    fn cell_at(&self, x: f32, z: f32) -> Option<usize> {
        if !self.on_map(x, z) {
            return None;
        }
        let column = ((x / CELL_SIDE) as usize).min(self.width - 1);
        let row = ((z / CELL_SIDE) as usize).min(self.height - 1);
        Some(row * self.width + column)
    }

    /// The ground under a world point; off the map, its edge.
    pub fn ground_at(&self, x: f32, z: f32) -> Ground {
        self.cell_at(x, z)
            .map_or(Ground::MapEdge, |cell| self.grounds[cell])
    }

    /// Whether a machine may stand here: on the map, on ground that can be driven over.
    pub fn can_walk(&self, x: f32, z: f32) -> bool {
        self.can_go(Going::Driving, x, z)
    }

    /// Whether someone going this way may be here. Aircraft only have to stay on the map.
    pub fn can_go(&self, going: Going, x: f32, z: f32) -> bool {
        match going {
            Going::Flying => self.on_map(x, z),
            _ => self.ground_at(x, z).lets(going),
        }
    }

    /// Whether a place is on rock, where buildings may go up.
    pub fn is_rock(&self, x: f32, z: f32) -> bool {
        self.ground_at(x, z) == Ground::Rock
    }

    pub fn on_map(&self, x: f32, z: f32) -> bool {
        let (right, bottom) = self.extent();
        (0.0..=right).contains(&x) && (0.0..=bottom).contains(&z)
    }

    /// A point anywhere on the map.
    pub fn random_point(&self, random: &mut u32) -> (f32, f32) {
        let (right, bottom) = self.extent();
        (next_random(random) * right, next_random(random) * bottom)
    }

    /// Whether a place has spice.
    pub fn has_substance(&self, x: f32, z: f32) -> bool {
        self.cell_at(x, z).is_some_and(|cell| self.spice[cell] > 0)
    }

    /// How much spice the cell at column `u` and row `v` has, from 0 to `FULL_SPICE`.
    pub fn spice_in(&self, u: usize, v: usize) -> u8 {
        if u < self.width && v < self.height {
            self.spice[v * self.width + u]
        } else {
            0
        }
    }

    /// The middle of the cell of spice nearest to a place, if the map has any.
    pub fn nearest_field(&self, (x, z): (f32, f32)) -> Option<(f32, f32)> {
        let middle = |cell: usize| {
            let (u, v) = (cell % self.width, cell / self.width);
            ((u as f32 + 0.5) * CELL_SIDE, (v as f32 + 0.5) * CELL_SIDE)
        };
        let away = |&(cx, cz): &(f32, f32)| (cx - x).hypot(cz - z);
        (0..self.spice.len())
            .filter(|&cell| self.spice[cell] > 0)
            .map(middle)
            .min_by(|a, b| away(a).total_cmp(&away(b)))
    }

    /// The middles of the cells with spice within a square of this reach about a point.
    pub fn fields_within(&self, (x, z): (f32, f32), reach: f32) -> Vec<(f32, f32)> {
        let span = |at: f32, cells: usize| {
            let first = ((at - reach) / CELL_SIDE).floor().max(0.0) as usize;
            let last = ((at + reach) / CELL_SIDE).ceil().max(0.0) as usize;
            first..last.min(cells)
        };
        span(z, self.height)
            .flat_map(|v| span(x, self.width).map(move |u| (u, v)))
            .filter(|&(u, v)| self.spice[v * self.width + u] > 0)
            .map(|(u, v)| ((u as f32 + 0.5) * CELL_SIDE, (v as f32 + 0.5) * CELL_SIDE))
            .collect()
    }

    /// How the map is lit.
    pub fn light(&self) -> Light {
        self.light
    }

    /// The color of a cell on the radar.
    pub fn radar_at(&self, u: usize, v: usize) -> [f32; 3] {
        self.radar[v * self.width + u]
    }

    /// Which grid points a route may go through: those all of whose cells can be gone over.
    fn route_grid(&self, going: Going) -> &[bool] {
        self.walkable[going as usize].get_or_init(|| {
            let mut grid = vec![false; GRID_POINTS];
            for v in 0..=self.height {
                for u in 0..=self.width {
                    let column = |u: usize| u.min(self.width - 1);
                    let row = |v: usize| v.min(self.height - 1);
                    let cells = [
                        (column(u.saturating_sub(1)), row(v.saturating_sub(1))),
                        (column(u), row(v.saturating_sub(1))),
                        (column(u.saturating_sub(1)), row(v)),
                        (column(u), row(v)),
                    ];
                    grid[grid_index(u, v)] = cells
                        .iter()
                        .all(|&(c, r)| self.grounds[r * self.width + c].lets(going));
                }
            }
            grid
        })
    }

    /// Calls back with each grid point on the map within a square around a place.
    fn for_points_near(&self, x: f32, z: f32, reach: f32, mut visit: impl FnMut(usize, f32, f32)) {
        let (u, v) = world_to_grid(x, z);
        let span = reach / CELL_SIDE + 1.0;
        let range = |center: f32, last: usize| {
            ((center - span).floor().max(0.0) as usize)
                ..=((center + span).ceil().max(0.0) as usize).min(last)
        };
        for point_v in range(v, self.height) {
            for point_u in range(u, self.width) {
                let (px, pz) = grid_to_world(point_u as f32, point_v as f32);
                visit(grid_index(point_u, point_v), px, pz);
            }
        }
    }

    /// Whether someone could walk straight from one point to the other. What lies right at
    /// either end is no matter: it may well stand beside a cliff.
    pub fn line_is_clear(&self, going: Going, from: (f32, f32), to: (f32, f32)) -> bool {
        self.line_is_clear_on(going, self.route_grid(going), from, to)
    }

    /// Whether the grid's points along a line are all open, and the ground itself can be walked
    /// along it with room for a vehicle each side: a line between two open points can still cut
    /// across the corner of a cliff between them.
    fn line_is_clear_on(
        &self,
        going: Going,
        grid: &[bool],
        from: (f32, f32),
        to: (f32, f32),
    ) -> bool {
        const STEP: f32 = 9.051;
        const ENDS: f32 = 33.94;
        /// How far each side of the line the ground has to be open too.
        const ROOM: f32 = 20.36;
        let length = (to.0 - from.0).hypot(to.1 - from.1);
        let steps = (length / STEP).ceil().max(1.0) as usize;
        let (side_x, side_z) = (
            -(to.1 - from.1) / length.max(0.004525) * ROOM,
            (to.0 - from.0) / length.max(0.004525) * ROOM,
        );
        (1..=steps).all(|i| {
            let t = i as f32 / steps as f32;
            let along = t * length;
            if along < ENDS || length - along < ENDS {
                return true;
            }
            let (x, z) = (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
            let (u, v) = world_to_grid(x, z);
            let (u, v) = (u.round(), v.round());
            u >= 0.0
                && v >= 0.0
                && u <= self.width as f32
                && v <= self.height as f32
                && grid[grid_index(u as usize, v as usize)]
                && [-1.0, 0.0, 1.0]
                    .iter()
                    .all(|&k| self.can_go(going, x + side_x * k, z + side_z * k))
        })
    }

    /// The nearest point to this one that can be walked on, if any is near enough to matter.
    pub fn nearest_walkable(&self, going: Going, (x, z): (f32, f32)) -> Option<(f32, f32)> {
        const RINGS: usize = 40;
        const PER_RING: usize = 24;
        if self.can_go(going, x, z) {
            return Some((x, z));
        }
        (1..=RINGS).find_map(|ring| {
            let reach = ring as f32 * CELL_SIDE * FRAC_1_SQRT_2;
            (0..PER_RING).find_map(|i| {
                let (sin, cos) = (i as f32 / PER_RING as f32 * std::f32::consts::TAU).sin_cos();
                let spot = (x + cos * reach, z + sin * reach);
                self.can_go(going, spot.0, spot.1).then_some(spot)
            })
        })
    }

    /// The first place to head for on the way around whatever lies between two points: as far
    /// along the shortest route as can be reached in a straight line. `None` if there is no
    /// way there at all.
    pub fn next_on_route(
        &self,
        going: Going,
        from: (f32, f32),
        to: (f32, f32),
    ) -> Option<(f32, f32)> {
        self.next_on_grid(going, self.route_grid(going), from, to)
    }

    /// As `next_on_route`, keeping clear of these as well, each given as (x, z, how far).
    pub fn next_on_route_around(
        &self,
        going: Going,
        from: (f32, f32),
        to: (f32, f32),
        obstacles: &[(f32, f32, f32)],
    ) -> Option<(f32, f32)> {
        let mut grid = self.route_grid(going).to_vec();
        for &(x, z, reach) in obstacles {
            self.for_points_near(x, z, reach, |index, px, pz| {
                if (px - x).hypot(pz - z) < reach {
                    grid[index] = false;
                }
            });
        }
        self.next_on_grid(going, &grid, from, to)
    }

    fn next_on_grid(
        &self,
        going: Going,
        grid_walkable: &[bool],
        from: (f32, f32),
        to: (f32, f32),
    ) -> Option<(f32, f32)> {
        const STRAIGHT: u32 = 10;
        const DIAGONAL: u32 = 14;
        const NOWHERE: usize = usize::MAX;
        let (last_u, last_v) = (self.width as i32, self.height as i32);
        let walkable = |index: usize| grid_walkable[index];
        let grid = |index: usize| {
            (
                (index % (MOST_CELLS + 1)) as i32,
                (index / (MOST_CELLS + 1)) as i32,
            )
        };
        // The walkable grid point nearest a world point, truly nearest: by a square ring at
        // a time, one across a cliff's corner could come before one just beside it.
        let snap = |(x, z): (f32, f32)| -> Option<usize> {
            const REACH: i32 = 4;
            let (u, v) = world_to_grid(x, z);
            let (u, v) = (u.round() as i32, v.round() as i32);
            (-REACH..=REACH)
                .flat_map(|du| (-REACH..=REACH).map(move |dv| (u + du, v + dv)))
                .filter(|&(u, v)| u >= 0 && v >= 0 && u <= last_u && v <= last_v)
                .map(|(u, v)| grid_index(u as usize, v as usize))
                .filter(|&index| walkable(index))
                .min_by(|&a, &b| {
                    let away = |index: usize| {
                        let (u, v) = grid(index);
                        let (px, pz) = grid_to_world(u as f32, v as f32);
                        (px - x).hypot(pz - z)
                    };
                    away(a).total_cmp(&away(b))
                })
        };
        let (start, goal) = (snap(from)?, snap(to)?);
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
                if nu < 0 || nv < 0 || nu > last_u || nv > last_v {
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
            .find(|&spot| self.line_is_clear_on(going, grid_walkable, from, spot))
            .or_else(|| route.iter().rev().nth(1).map(|&index| point(index)))
    }

    /// The lowest ground anyone stands on: the original's sand lies at height 0.
    pub fn floor(&self) -> f32 {
        0.0
    }

    /// The terrain's height under a world point, between the heights at the corners of its cell.
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let (u, v) = world_to_grid(x, z);
        let (last_u, last_v) = (self.width as f32, self.height as f32);
        let (u, v) = (u.clamp(0.0, last_u), v.clamp(0.0, last_v));
        let (u0, v0) = (u.floor().min(last_u - 1.0), v.floor().min(last_v - 1.0));
        let (tu, tv) = (u - u0, v - v0);
        let at =
            |du: usize, dv: usize| self.heights[grid_index(u0 as usize + du, v0 as usize + dv)];
        let near = at(0, 0) * (1.0 - tu) + at(1, 0) * tu;
        let far = at(0, 1) * (1.0 - tu) + at(1, 1) * tu;
        near * (1.0 - tv) + far * tv
    }
}

/// A color as the radar picture keeps it, in five bits of red, six of green and five of blue.
fn rgb565(pixel: u16) -> [f32; 3] {
    [
        f32::from(pixel >> 11) / 31.0,
        f32::from((pixel >> 5) & 63) / 63.0,
        f32::from(pixel & 31) / 31.0,
    ]
}

/// The original's map of this number in the list the disc gives, read the first time it is
/// needed. One that cannot be read is blank sand.
pub fn terrain(map: usize) -> &'static Terrain {
    static TERRAINS: OnceLock<Vec<OnceLock<Terrain>>> = OnceLock::new();
    static BLANK: OnceLock<Terrain> = OnceLock::new();
    let terrains = TERRAINS.get_or_init(|| {
        (0..crate::maps::list().len())
            .map(|_| OnceLock::new())
            .collect()
    });
    let blank = || BLANK.get_or_init(|| Terrain::blank(MOST_CELLS, MOST_CELLS));
    terrains.get(map).map_or_else(blank, |terrain| {
        terrain.get_or_init(|| {
            let light = crate::maps::file(map, "test.lit")
                .and_then(|text| Light::read(&String::from_utf8_lossy(&text)))
                .unwrap_or(DAYLIGHT);
            crate::maps::file(map, "test.xbf")
                .and_then(|file| {
                    Terrain::read(&file, crate::maps::file(map, "test.CPF").as_deref())
                })
                .unwrap_or_else(|| Terrain::blank(MOST_CELLS, MOST_CELLS))
                .lit(light)
        })
    })
}

/// What the backend makes of a distance `d` from the camera as a depth from 0 to 1: a + b / d.
fn depth_terms() -> (f32, f32) {
    (FAR / (FAR - NEAR), -NEAR * FAR / (FAR - NEAR))
}

/// A point on the ground as how far across the screen and down it, the camera's turn taken off.
pub fn across_and_down(x: f32, z: f32) -> (f32, f32) {
    (
        x * SCREEN_RIGHT.0 + z * SCREEN_RIGHT.1,
        x * SCREEN_DOWN.0 + z * SCREEN_DOWN.1,
    )
}

/// The world point that is this far across the screen and down it.
pub fn from_across_and_down(across: f32, down: f32) -> (f32, f32) {
    (
        across * SCREEN_RIGHT.0 + down * SCREEN_DOWN.0,
        across * SCREEN_RIGHT.1 + down * SCREEN_DOWN.1,
    )
}

pub fn grid_to_world(u: f32, v: f32) -> (f32, f32) {
    (u * CELL_SIDE, v * CELL_SIDE)
}

pub fn world_to_grid(x: f32, z: f32) -> (f32, f32) {
    (x / CELL_SIDE, z / CELL_SIDE)
}

/// The ground point the camera looks at, and how far it has zoomed: 0 is the top position
/// with the widest field of view, 1 the bottom position with the narrowest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub x: f32,
    pub z: f32,
    pub zoom: f32,
    zoom_target: f32,
    /// How far the map it is kept on reaches along x and along z.
    extent: (f32, f32),
}

impl Camera {
    /// Looking at the middle of the biggest map from the top position.
    pub fn centered() -> Self {
        let side = MOST_CELLS as f32 * CELL_SIDE;
        Self::centered_on((side, side))
    }

    /// Looking at the middle of a map reaching this far along x and z, from the top position.
    pub fn centered_on(extent: (f32, f32)) -> Self {
        Camera {
            x: extent.0 / 2.0,
            z: extent.1 / 2.0,
            zoom: 0.0,
            zoom_target: 0.0,
            extent,
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

    /// Moves the target, sliding along the map's edges rather than leaving it.
    fn move_by(&mut self, dx: f32, dz: f32) {
        self.x = (self.x + dx).clamp(0.0, self.extent.0);
        self.z = (self.z + dz).clamp(0.0, self.extent.1);
    }

    /// One frame of edge scrolling, `direction` given on the screen. Slower when low, so the
    /// ground crosses the screen at about the same pace from either position.
    pub fn scroll(&mut self, direction: (f32, f32), scroll_rate: i32, dt: f32) {
        let pace = self.altitude() / TOP_HEIGHT / self.lens();
        let distance = SCROLL_PER_RATE * scroll_rate as f32 * pace * dt;
        let (right, down) = (direction.0 * distance, direction.1 * distance);
        self.move_by(
            right * SCREEN_RIGHT.0 + down * SCREEN_DOWN.0,
            right * SCREEN_RIGHT.1 + down * SCREEN_DOWN.1,
        );
    }

    /// How far between the two positions: 0 at the top, 1 at the bottom.
    fn descent(self) -> f32 {
        self.zoom
    }

    fn altitude(self) -> f32 {
        TOP_HEIGHT + (BOTTOM_HEIGHT - TOP_HEIGHT) * self.descent()
    }

    /// Magnification from narrowing the field of view: 1 at the top, growing on the way down.
    fn lens(self) -> f32 {
        1.0 + (MAX_LENS - 1.0) * self.zoom
    }
}

/// A camera pose ready for projecting, for one frame on one screen.
pub struct View {
    position: [f32; 3],
    /// Unit vectors in the plane of up and the screen's bottom on the ground: the camera never
    /// rolls.
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
        let back = altitude * cos / sin;
        View {
            position: [
                camera.x + SCREEN_DOWN.0 * back,
                altitude,
                camera.z + SCREEN_DOWN.1 * back,
            ],
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
        let (r0, r1) = SCREEN_RIGHT;
        let (d0, d1) = SCREEN_DOWN;
        let side = r0 * px + r1 * pz;
        let back = d0 * px + d1 * pz;
        let along = f0 * py + f1 * back;
        [
            sx * r0,
            sy * u1 * d0,
            a * f1 * d0,
            f1 * d0,
            0.0,
            sy * u0,
            a * f0,
            f0,
            sx * r1,
            sy * u1 * d1,
            a * f1 * d1,
            f1 * d1,
            -sx * side,
            -sy * (u0 * py + u1 * back),
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
    pub fn sun_matrix(
        &self,
        focus: (f32, f32),
        sun: [f32; 3],
        width: f32,
        height: f32,
    ) -> [f32; 16] {
        /// The pixels across the backend's shadow map.
        const SHADOW_MAP: f32 = 4096.0;
        const MOST_REACH: f32 = 4073.0;
        const ROOM: f32 = 362.0;
        /// How far along the sun's rays it takes in.
        const THROUGH: f32 = 4073.0;
        let reach = self.ground_seen(width, height).map_or(MOST_REACH, |seen| {
            let far_x = (seen.0 - focus.0).abs().max((seen.2 - focus.0).abs());
            let far_z = (seen.1 - focus.1).abs().max((seen.3 - focus.1).abs());
            (far_x.hypot(far_z) + ROOM).min(MOST_REACH)
        });
        // In whole steps, so that it does not change size with every small move.
        let reach = (reach / 181.0).ceil() * 181.0;
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
        let sun = unit(sun);
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
        let (dx, rel_y, dz) = (x - px, y - py, z - pz);
        let side = dx * SCREEN_RIGHT.0 + dz * SCREEN_RIGHT.1;
        let back = dx * SCREEN_DOWN.0 + dz * SCREEN_DOWN.1;
        let up = rel_y * self.up[0] + back * self.up[1];
        (side, up, rel_y * self.forward[0] + back * self.forward[1])
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
        let limit = NEAR + 0.4525;
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

    /// The world (x, z) on the land itself under a screen point, up on what stands high too.
    pub fn land_at(&self, terrain: &Terrain, screen_x: f32, screen_y: f32) -> Option<(f32, f32)> {
        let mut at = self.ground_at(screen_x, screen_y)?;
        for _ in 0..4 {
            at = self.plane_at(screen_x, screen_y, terrain.height_at(at.0, at.1))?;
        }
        Some(at)
    }

    /// The world (x, z) on the sand plane under a screen point; `None` above the horizon.
    pub fn ground_at(&self, screen_x: f32, screen_y: f32) -> Option<(f32, f32)> {
        self.plane_at(screen_x, screen_y, 0.0)
    }

    /// The world (x, z) at some height under a screen point; `None` above the horizon.
    fn plane_at(&self, screen_x: f32, screen_y: f32, height: f32) -> Option<(f32, f32)> {
        let side = (screen_x - self.center.0) / self.focal;
        let lift = (self.center.1 - screen_y) / self.focal;
        let ray_y = self.forward[0] + lift * self.up[0];
        let ray_back = self.forward[1] + lift * self.up[1];
        if ray_y >= 0.0 {
            return None;
        }
        let [px, py, pz] = self.position;
        let t = (height - py) / ray_y;
        let (side, back) = (side * t, ray_back * t);
        Some((
            px + SCREEN_RIGHT.0 * side + SCREEN_DOWN.0 * back,
            pz + SCREEN_RIGHT.1 * side + SCREEN_DOWN.1 * back,
        ))
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

/// How far across the map shows on the minimap, turned as the camera turns it: the square a
/// map reaching this far along x and z fits in, as a diamond.
pub fn minimap_span((width, height): (f32, f32)) -> f32 {
    (width + height) * FRAC_1_SQRT_2
}

/// A world point's place on a minimap square showing a map reaching this far, turned as the
/// camera turns it: the map's first corner at the top.
pub fn world_to_minimap(
    (x, y, size): (f32, f32, f32),
    extent: (f32, f32),
    world_x: f32,
    world_z: f32,
) -> (f32, f32) {
    let scale = size / minimap_span(extent);
    let across = (world_x - world_z) * FRAC_1_SQRT_2 + extent.1 * FRAC_1_SQRT_2;
    let down = (world_x + world_z) * FRAC_1_SQRT_2;
    (x + across * scale, y + down * scale)
}

/// The world point under a minimap position, or `None` outside the minimap's square.
pub fn minimap_to_world(
    (x, y, size): (f32, f32, f32),
    extent: (f32, f32),
    px: f32,
    py: f32,
) -> Option<(f32, f32)> {
    if px < x || px >= x + size || py < y || py >= y + size {
        return None;
    }
    let scale = minimap_span(extent) / size;
    let across = (px - x) * scale - extent.1 * FRAC_1_SQRT_2;
    let down = (py - y) * scale;
    Some((
        (across + down) * FRAC_1_SQRT_2,
        (down - across) * FRAC_1_SQRT_2,
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

/// Which way the cursor pushes the camera on the screen, with length 1 or 0. Corners push
/// diagonally.
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

/// How bright the land is at a world point, as the player knows it.
pub fn seen_at(fog: Option<&Fog>, x: f32, z: f32) -> f32 {
    fog.map_or(1.0, |fog| fog.at(x, z).shade())
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f32 = 2880.0;
    const H: f32 = 1800.0;

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

    /// A map's file as the original writes one: its version, the length of its parts, and
    /// the parts, with nothing of its shape after them.
    fn map_file(
        width: u32,
        height: u32,
        grounds: &[u8],
        spice: &[u8],
        bases: &[(f64, f64)],
    ) -> Vec<u8> {
        let mut parts = Vec::new();
        let mut part = |tag: u32, data: &[u8]| {
            parts.extend(tag.to_le_bytes());
            parts.extend((data.len() as u32).to_le_bytes());
            parts.extend(data);
        };
        let size: Vec<u8> = [width, height]
            .iter()
            .flat_map(|n| n.to_le_bytes())
            .collect();
        part(SIZE, &size);
        let heights: Vec<u8> = (0..(width + 1) * (height + 1))
            .flat_map(|n| (n as f32).to_le_bytes())
            .collect();
        part(HEIGHTS, &heights);
        part(GROUNDS, grounds);
        part(SPICE, spice);
        let name = |name: &[u8]| {
            let mut field = name.to_vec();
            field.resize(AREA_NAME, 0);
            field
        };
        let mut areas = 2u32.to_le_bytes().to_vec();
        areas.extend(name(b"Mission"));
        areas.extend(0u32.to_le_bytes());
        areas.extend(name(BASES));
        areas.extend(2u32.to_le_bytes());
        for (list, points) in [
            (&b"Primary"[..], &bases[1..]),
            (&b"Default"[..], &bases[..1]),
        ] {
            areas.extend(name(list));
            areas.extend((points.len() as u32).to_le_bytes());
            for &(x, z) in points {
                areas.extend((-1i32).to_le_bytes());
                areas.extend(x.to_le_bytes());
                areas.extend(z.to_le_bytes());
            }
        }
        part(AREAS, &areas);
        let mut file = 1u32.to_le_bytes().to_vec();
        file.extend((parts.len() as u32).to_le_bytes());
        file.extend(parts);
        file
    }

    #[test]
    fn a_map_is_read_as_the_original_keeps_it() {
        // Three columns and two rows: rock, cliff, sand, then ramp, spice-laden sand, map edge.
        let file = map_file(
            3,
            2,
            &[1, 2, 0, 7, 0, 6],
            &[0, 0, 0, 0, 255, 0],
            &[(16.0, 48.0), (80.0, 16.0)],
        );
        let terrain = Terrain::read(&file, None).unwrap();
        assert_eq!(terrain.cells(), (3, 2));
        assert_eq!(terrain.extent(), (96.0, 64.0));
        assert_eq!(terrain.ground_at(16.0, 16.0), Ground::Rock);
        assert_eq!(terrain.ground_at(48.0, 16.0), Ground::Cliff);
        assert_eq!(terrain.ground_at(16.0, 48.0), Ground::Ramp);
        assert_eq!(terrain.ground_at(80.0, 48.0), Ground::MapEdge);
        assert!(terrain.is_rock(16.0, 16.0) && !terrain.is_rock(80.0, 16.0));
        assert!(!terrain.can_walk(48.0, 16.0) && terrain.can_walk(80.0, 16.0));
        assert!(terrain.has_substance(48.0, 48.0) && !terrain.has_substance(16.0, 48.0));
        assert_eq!(
            terrain.starts(),
            [(80.0, 16.0), (16.0, 48.0)],
            "its primary bases, then its default"
        );
        assert_eq!(terrain.height_at(32.0, 0.0), 1.0, "heights go row by row");
        assert_eq!(terrain.height_at(0.0, 32.0), 4.0);
        assert_eq!(terrain.height_at(16.0, 0.0), 0.5, "and between them");
    }

    #[test]
    fn a_route_goes_around_cliff_and_there_is_none_through_it() {
        let terrain = Terrain::blank(40, 40).with_ground(Ground::Cliff, (20, 0), (1, 30));
        let (from, to) = (
            (10.0 * CELL_SIDE, 10.0 * CELL_SIDE),
            (30.0 * CELL_SIDE, 10.0 * CELL_SIDE),
        );
        assert!(!terrain.line_is_clear(Going::OnFoot, from, to));
        let next = terrain.next_on_route(Going::OnFoot, from, to).unwrap();
        assert!(
            next.1 > 29.0 * CELL_SIDE,
            "down past the end of the cliff: {next:?}"
        );
        let walled = terrain.with_ground(Ground::Cliff, (20, 30), (1, 10));
        assert_eq!(walled.next_on_route(Going::OnFoot, from, to), None);
    }

    #[test]
    fn infantry_climb_the_stairs_of_their_rock_that_machines_drive_around() {
        let terrain = Terrain::blank(40, 40).with_ground(Ground::InfRock, (20, 0), (1, 30));
        let (from, to) = (
            (10.0 * CELL_SIDE, 10.0 * CELL_SIDE),
            (30.0 * CELL_SIDE, 10.0 * CELL_SIDE),
        );
        assert!(terrain.line_is_clear(Going::OnFoot, from, to));
        assert!(!terrain.line_is_clear(Going::Driving, from, to));
        let next = terrain.next_on_route(Going::Driving, from, to).unwrap();
        assert!(next.1 > 29.0 * CELL_SIDE, "around the end: {next:?}");
        let (x, z) = (20.5 * CELL_SIDE, 10.0 * CELL_SIDE);
        assert!(terrain.can_go(Going::OnFoot, x, z) && !terrain.can_walk(x, z));
    }

    #[test]
    fn a_map_is_lit_as_its_light_file_says_with_its_z_turned_to_ours() {
        let light = Light::read("1 1.8 1\r\n74 67 56\r\n242 178 127\r\n55 45 33\r\n").unwrap();
        let length = (1.0f32 + 1.8 * 1.8 + 1.0).sqrt();
        assert_eq!(light.sun, [1.0 / length, 1.8 / length, -1.0 / length]);
        assert_eq!(light.ambient, [74.0 / 255.0, 67.0 / 255.0, 56.0 / 255.0]);
        assert_eq!(
            light.sunlight,
            [242.0 / 255.0, 178.0 / 255.0, 127.0 / 255.0]
        );
        assert_eq!(Light::read("1 1\n2 2 2\n3 3 3"), None);
    }

    #[test]
    fn spice_is_found_nearest_first() {
        let rock = Terrain::blank(40, 40).with_ground(Ground::Rock, (0, 0), (40, 40));
        let terrain = rock.with_spice((30, 30), (2, 2)).with_spice((5, 5), (1, 1));
        assert_eq!(
            terrain.nearest_field((0.0, 0.0)),
            Some((5.5 * CELL_SIDE, 5.5 * CELL_SIDE))
        );
        assert_eq!(Terrain::blank(4, 4).nearest_field((0.0, 0.0)), None);
    }

    #[test]
    fn zoom_has_two_fixed_positions_with_the_requested_tilt() {
        assert!(
            (pitch(at_zoom(0.0)) - 75.0).abs() < 0.01,
            "top looks almost straight down"
        );
        assert!(
            (pitch(at_zoom(1.0)) - 30.0).abs() < 0.01,
            "bottom is 30 degrees from the ground"
        );
        assert!(
            View::new(at_zoom(1.0), W, H).position[1] < View::new(at_zoom(0.0), W, H).position[1]
        );
    }

    #[test]
    fn zooming_keeps_the_same_ground_point_at_screen_center() {
        for zoom in [0.0, 0.3, 0.5, 0.7, 1.0] {
            let camera = at_zoom(zoom);
            let (x, y) = View::new(camera, W, H)
                .project(camera.x, 0.0, camera.z)
                .unwrap();
            assert!(
                near(x, W / 2.0, 0.01) && near(y, H / 2.0, 0.01),
                "at {zoom}"
            );
        }
    }

    #[test]
    fn the_map_shows_as_a_diamond_with_its_first_corner_at_the_top() {
        let camera = Camera::centered();
        let view = View::new(camera, W, H);
        let at = |dx: f32, dz: f32| view.project(camera.x + dx, 0.0, camera.z + dz).unwrap();
        let (top, bottom) = (at(-200.0, -200.0), at(200.0, 200.0));
        let (right, left) = (at(200.0, -200.0), at(-200.0, 200.0));
        assert!(
            top.1 < right.1 && right.1 < bottom.1,
            "first corner up, last down"
        );
        assert!(near(top.0, W / 2.0, 0.5) && near(bottom.0, W / 2.0, 0.5));
        assert!(
            left.0 < W / 2.0 && W / 2.0 < right.0,
            "columns run down to the right"
        );
        assert!(near(right.1, left.1, 0.5));
    }

    #[test]
    fn ground_at_reverses_projection() {
        for zoom in [0.0, 0.5, 1.0] {
            let camera = at_zoom(zoom);
            let view = View::new(camera, W, H);
            let (x, z) = (camera.x + 226.3, camera.z - 135.8);
            let (sx, sy) = view.project(x, 0.0, z).unwrap();
            let (gx, gz) = view.ground_at(sx, sy).unwrap();
            assert!(near(gx, x, 0.4525) && near(gz, z, 0.4525), "at {zoom}");
        }
    }

    #[test]
    fn the_matrix_the_backend_gets_lands_on_the_same_pixels_and_depth_as_the_game_works_with() {
        for zoom in [0.0, 0.5, 1.0] {
            let camera = at_zoom(zoom);
            let view = View::new(camera, W, H);
            let (x, z) = (camera.x, camera.z);
            for point in [
                (x, 0.0, z),
                (x + 135.8, 90.51, z - 181.0),
                (x - 271.5, 31.68, z + 45.25),
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
    fn a_mirror_shows_what_is_above_it_lower_on_the_screen_and_what_is_at_it_in_place() {
        let camera = Camera::centered();
        let view = View::new(camera, W, H);
        let above = through(view.matrix(), (camera.x, 90.51, camera.z));
        let reflected = through(view.mirror_matrix(), (camera.x, 90.51, camera.z));
        assert!(reflected.1 > above.1);
        let at_it = through(view.matrix(), (camera.x, 0.0, camera.z));
        let at_it_reflected = through(view.mirror_matrix(), (camera.x, 0.0, camera.z));
        assert!(near(at_it.1, at_it_reflected.1, 0.01));
    }

    #[test]
    fn the_sun_sees_the_camera_area_with_what_is_nearer_it_at_less_depth() {
        let camera = at_zoom(0.0);
        let view = View::new(camera, W, H);
        let sun_view = view.sun_matrix((camera.x, camera.z), SUN, W, H);
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
        let (_, _, high) = at(226.3);
        assert!(high < ground, "nearer the sun");
        assert!(ground > 0.0 && ground < 1.0);
    }

    #[test]
    fn scrolling_moves_the_way_the_screen_is_pushed_and_slows_as_the_view_narrows() {
        let moved = |zoom: f32, direction: (f32, f32)| {
            let mut camera = at_zoom(zoom);
            camera.scroll(direction, 5, 0.1);
            let view = View::new(at_zoom(zoom), W, H);
            let (sx, sy) = view.project(camera.x, 0.0, camera.z).unwrap();
            (sx - W / 2.0, sy - H / 2.0)
        };
        let right = moved(0.0, (1.0, 0.0));
        assert!(right.0 > 0.0 && near(right.1, 0.0, 0.5), "{right:?}");
        let down = moved(0.0, (0.0, 1.0));
        assert!(down.1 > 0.0 && near(down.0, 0.0, 0.5), "{down:?}");
        assert!(moved(1.0, (1.0, 0.0)).0 < right.0 * 2.0);
    }

    #[test]
    fn the_camera_stays_on_the_map_and_slides_along_its_edges() {
        let mut camera = Camera::centered_on((3200.0, 1600.0));
        assert_eq!(
            (camera.x, camera.z),
            (1600.0, 800.0),
            "starts in its middle"
        );
        for _ in 0..400 {
            camera.scroll((0.0, -1.0), 10, 0.1);
        }
        assert_eq!((camera.x, camera.z), (0.0, 0.0), "up into its first corner");
        for _ in 0..400 {
            camera.scroll((1.0, 0.0), 10, 0.1);
        }
        assert_eq!(
            (camera.x, camera.z),
            (3200.0, 0.0),
            "then right along its edge"
        );
        camera.look_at(-50.0, 99999.0);
        assert_eq!((camera.x, camera.z), (0.0, 1600.0));
    }

    #[test]
    fn minimap_positions_put_the_map_in_a_diamond_and_round_trip() {
        let rect = (1000.0, 100.0, 400.0);
        let extent = (3200.0, 3200.0);
        let corner = |x, z| world_to_minimap(rect, extent, x, z);
        let near_by = |(a, b): (f32, f32), (c, d): (f32, f32)| near(a, c, 0.01) && near(b, d, 0.01);
        assert!(
            near_by(corner(0.0, 0.0), (1200.0, 100.0)),
            "first corner top middle"
        );
        assert!(
            near_by(corner(3200.0, 3200.0), (1200.0, 500.0)),
            "last bottom middle"
        );
        assert!(
            near_by(corner(3200.0, 0.0), (1400.0, 300.0)),
            "first row to the right"
        );
        assert!(
            near_by(corner(0.0, 3200.0), (1000.0, 300.0)),
            "first column to the left"
        );
        let (x, z) = minimap_to_world(rect, extent, 1250.0, 260.0).unwrap();
        assert!(near_by(
            world_to_minimap(rect, extent, x, z),
            (1250.0, 260.0)
        ));
        assert_eq!(minimap_to_world(rect, extent, 999.0, 300.0), None);
        let wide = (6400.0, 3200.0);
        let (x, z) = minimap_to_world(rect, wide, 1100.0, 380.0).unwrap();
        assert!(near_by(world_to_minimap(rect, wide, x, z), (1100.0, 380.0)));
    }

    #[test]
    fn segments_clip_to_a_rectangle_and_a_square() {
        let area = (0.0, 0.0, 20.0, 10.0);
        assert_eq!(
            clip_to_rect((-5.0, 5.0), (25.0, 5.0), area),
            Some(((0.0, 5.0), (20.0, 5.0)))
        );
        assert_eq!(clip_to_rect((5.0, 12.0), (15.0, 12.0), area), None);
        assert_eq!(
            clip_to_square((5.0, -5.0), (5.0, 5.0), (0.0, 0.0, 10.0)),
            Some(((5.0, 0.0), (5.0, 5.0)))
        );
    }

    #[test]
    fn edges_push_their_way_and_corners_push_diagonally() {
        let d = FRAC_1_SQRT_2;
        let (r, b) = (W - 1.0, H - 1.0);
        for (mouse, expected) in [
            ((W / 2.0, H / 2.0), (0.0, 0.0)),
            ((0.0, H / 2.0), (-1.0, 0.0)),
            ((r, H / 2.0), (1.0, 0.0)),
            ((W / 2.0, 0.0), (0.0, -1.0)),
            ((W / 2.0, b), (0.0, 1.0)),
            ((0.0, 0.0), (-d, -d)),
            ((r, b), (d, d)),
        ] {
            let got = edge_direction(mouse, W, H);
            assert!(
                near(got.0, expected.0, 1e-5) && near(got.1, expected.1, 1e-5),
                "{mouse:?} gave {got:?}"
            );
        }
    }
}
