//! The 3D battlefield: terrain and camera, projected on the CPU.
//!
//! Terrain is drawn back to front, so it needs no depth buffer and the graphics backends
//! still only receive colored triangles.
//!
//! The map is a square grid turned 45 degrees into a diamond. Grid coordinates (u, v) run
//! 0..=MAP_CELLS: north is (0, 0), east (MAP_CELLS, 0), west (0, MAP_CELLS) and south
//! (MAP_CELLS, MAP_CELLS). World x points east, y up and z south.

use crate::planet::fractal_noise;
use crate::platform::Vertex;
use std::sync::OnceLock;

pub const MAP_CELLS: usize = 128;
/// World units per grid step along each world axis.
const CELL: f32 = 0.5;
/// How far ground rises above sand, in world units.
const GROUND_HEIGHT: f32 = 0.7;
/// Noise above this is ground, below it sand.
const GROUND_THRESHOLD: f32 = 0.54;

pub const SAND: [f32; 3] = [0.78, 0.64, 0.40];
pub const GROUND: [f32; 3] = [0.40, 0.30, 0.21];
/// How strongly the surface grain varies the color, either way.
const GRAIN: f32 = 0.22;

/// Camera height and how far south of its target it sits. This fixes its downward tilt.
const CAMERA_HEIGHT: f32 = 30.0;
const CAMERA_BACK: f32 = 22.0;
/// Vertical field of view, as the tangent of half its angle (about 40 degrees in total).
const HALF_FOV_TAN: f32 = 0.364;
const NEAR: f32 = 1.0;

/// Pixels from the screen edge that start scrolling, on a 1080-pixel-tall screen.
const EDGE_MARGIN: f32 = 4.0;
/// World units per second, per point of the Scroll Rate option.
const SCROLL_PER_RATE: f32 = 3.0;

pub struct Terrain {
    is_ground: Vec<bool>,
    colors: Vec<[f32; 3]>,
}

fn grid_index(u: usize, v: usize) -> usize {
    v * (MAP_CELLS + 1) + u
}

impl Terrain {
    fn generate(map: usize) -> Self {
        let count = (MAP_CELLS + 1) * (MAP_CELLS + 1);
        let mut is_ground = Vec::with_capacity(count);
        let mut colors = Vec::with_capacity(count);
        let seed = 101 + map as u32 * 13;
        for v in 0..=MAP_CELLS {
            for u in 0..=MAP_CELLS {
                let (fu, fv) = (u as f32 / MAP_CELLS as f32, v as f32 / MAP_CELLS as f32);
                let height = fractal_noise([fu * 2.5, fv * 2.5, 0.5], seed);
                let ground = height > GROUND_THRESHOLD;
                let grain = fractal_noise([fu * 30.0, fv * 30.0, 2.5], seed + 500) - 0.5;
                let base = if ground { GROUND } else { SAND };
                is_ground.push(ground);
                colors.push(base.map(|c| c * (1.0 + grain * 2.0 * GRAIN)));
            }
        }
        Terrain { is_ground, colors }
    }

    pub fn is_ground(&self, u: usize, v: usize) -> bool {
        self.is_ground[grid_index(u, v)]
    }
}

/// The terrain of one map, generated the first time it is needed.
pub fn terrain(map: usize) -> &'static Terrain {
    static TERRAINS: [OnceLock<Terrain>; crate::game::MAPS.len()] =
        [const { OnceLock::new() }; crate::game::MAPS.len()];
    TERRAINS[map].get_or_init(|| Terrain::generate(map))
}

fn grid_to_world(u: f32, v: f32) -> (f32, f32) {
    ((u - v) * CELL, (u + v) * CELL)
}

fn world_to_grid(x: f32, z: f32) -> (f32, f32) {
    ((z + x) / (2.0 * CELL), (z - x) / (2.0 * CELL))
}

/// The ground point the camera looks at. Its tilt and height are fixed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub x: f32,
    pub z: f32,
}

impl Camera {
    /// Looking at the middle of the map.
    pub fn centered() -> Self {
        let middle = MAP_CELLS as f32 / 2.0;
        let (x, z) = grid_to_world(middle, middle);
        Camera { x, z }
    }

    /// Moves the target, sliding along the map's edges rather than leaving the diamond.
    fn move_by(&mut self, dx: f32, dz: f32) {
        let (u, v) = world_to_grid(self.x + dx, self.z + dz);
        let limit = MAP_CELLS as f32;
        (self.x, self.z) = grid_to_world(u.clamp(0.0, limit), v.clamp(0.0, limit));
    }

    /// One frame of edge scrolling.
    pub fn scroll(&mut self, direction: (f32, f32), scroll_rate: i32, dt: f32) {
        let distance = SCROLL_PER_RATE * scroll_rate as f32 * dt;
        self.move_by(direction.0 * distance, direction.1 * distance);
    }

    fn project(self, x: f32, y: f32, z: f32, width: f32, height: f32) -> Option<(f32, f32)> {
        let length = CAMERA_HEIGHT.hypot(CAMERA_BACK);
        let (rel_x, rel_y, rel_z) = (x - self.x, y - CAMERA_HEIGHT, z - (self.z + CAMERA_BACK));
        let depth = -(rel_y * CAMERA_HEIGHT + rel_z * CAMERA_BACK) / length;
        if depth < NEAR {
            return None;
        }
        let up = (rel_y * CAMERA_BACK - rel_z * CAMERA_HEIGHT) / length;
        let focal = height / (2.0 * HALF_FOV_TAN);
        Some((width / 2.0 + focal * rel_x / depth, height / 2.0 - focal * up / depth))
    }
}

/// Which way the cursor pushes the camera, with length 1 or 0. Corners push diagonally.
pub fn edge_direction(mouse: (f32, f32), width: f32, height: f32) -> (f32, f32) {
    let margin = EDGE_MARGIN * crate::game::ui_scale(height);
    let dx = if mouse.0 <= margin {
        -1.0
    } else if mouse.0 >= width - 1.0 - margin {
        1.0
    } else {
        0.0
    };
    let dz = if mouse.1 <= margin {
        -1.0
    } else if mouse.1 >= height - 1.0 - margin {
        1.0
    } else {
        0.0
    };
    let length = f32::hypot(dx, dz);
    if length > 0.0 { (dx / length, dz / length) } else { (0.0, 0.0) }
}

/// Appends the visible terrain, farthest first. Both grid axes point towards the camera,
/// so plain nested loops give that order.
pub fn draw(out: &mut Vec<Vertex>, terrain: &Terrain, camera: Camera, width: f32, height: f32) {
    let point = |u: usize, v: usize| {
        let index = grid_index(u, v);
        let (x, z) = grid_to_world(u as f32, v as f32);
        let y = if terrain.is_ground[index] { GROUND_HEIGHT } else { 0.0 };
        let [r, g, b] = terrain.colors[index];
        camera
            .project(x, y, z, width, height)
            .map(|(x, y)| Vertex { x, y, r, g, b, a: 1.0 })
    };
    for u in 0..MAP_CELLS {
        for v in 0..MAP_CELLS {
            let corners = [point(u, v), point(u + 1, v), point(u + 1, v + 1), point(u, v + 1)];
            let [Some(a), Some(b), Some(c), Some(d)] = corners else { continue };
            let quad = [a, b, c, d];
            let off_screen = quad.iter().all(|p| p.x < 0.0)
                || quad.iter().all(|p| p.x > width)
                || quad.iter().all(|p| p.y < 0.0)
                || quad.iter().all(|p| p.y > height);
            if !off_screen {
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
        camera.project(x, y, z, W, H).unwrap()
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
        assert!(north.1 < east.1 && east.1 < south.1, "north above, south below");
        assert!((north.0 - W / 2.0).abs() < 0.5 && (south.0 - W / 2.0).abs() < 0.5);
        assert!(west.0 < W / 2.0 && W / 2.0 < east.0, "west left, east right");
        assert!((east.1 - west.1).abs() < 0.5, "east and west are level");
    }

    #[test]
    fn the_camera_target_lands_at_screen_center() {
        let camera = Camera::centered();
        let (x, y) = camera.project(camera.x, 0.0, camera.z, W, H).unwrap();
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
        assert!(screen(camera, middle, middle, GROUND_HEIGHT).1 < screen(camera, middle, middle, 0.0).1);
        assert!(GROUND.iter().sum::<f32>() < SAND.iter().sum::<f32>());
    }

    #[test]
    fn every_map_has_sand_and_ground_and_each_map_differs() {
        let counts: Vec<Vec<bool>> = (0..crate::game::MAPS.len())
            .map(|map| {
                let terrain = terrain(map);
                let ground = terrain.is_ground.iter().filter(|g| **g).count();
                let total = terrain.is_ground.len();
                assert!(ground > total / 8 && ground < total * 2 / 3, "map {map}: {ground} of {total}");
                terrain.is_ground.clone()
            })
            .collect();
        for (i, a) in counts.iter().enumerate() {
            for b in &counts[i + 1..] {
                assert_ne!(a, b);
            }
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
            assert!((got.0 - expected.0).abs() < 1e-5 && (got.1 - expected.1).abs() < 1e-5, "{mouse:?} gave {got:?}");
        }
        assert_eq!(edge_direction((30.0, 30.0), W, H), (0.0, 0.0), "near is not at the edge");
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
        for direction in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0), (d, d), (-d, -d), (d, -d), (-d, d)] {
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
        assert!((u - LAST).abs() < 0.01 && v.abs() < 0.01, "ended at {u}, {v}");
    }

    #[test]
    fn draw_only_emits_what_is_on_screen_and_follows_the_camera() {
        let terrain = terrain(0);
        let mut centered = Vec::new();
        draw(&mut centered, terrain, Camera::centered(), W, H);
        assert!(!centered.is_empty());
        assert_eq!(centered.len() % 6, 0);
        let all_quads = MAP_CELLS * MAP_CELLS * 6;
        assert!(centered.len() < all_quads, "culling keeps it below the whole map");

        let mut moved = Vec::new();
        let mut camera = Camera::centered();
        camera.scroll((1.0, 0.0), 5, 0.5);
        draw(&mut moved, terrain, camera, W, H);
        assert_ne!(centered, moved);
    }

    #[test]
    fn draw_order_is_far_to_near() {
        let terrain = terrain(0);
        let mut out = Vec::new();
        draw(&mut out, terrain, Camera::centered(), W, H);
        // The first quad drawn is the farthest visible, so it sits higher on screen.
        let (first, last) = (out[0], out[out.len() - 1]);
        assert!(first.y < last.y);
    }
}
