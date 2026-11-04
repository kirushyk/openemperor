//! Rotating 3D planets with a generated surface and no lighting.
//!
//! The surface is computed once per sphere vertex and only the positions turn each frame,
//! so planets need nothing from the graphics backends beyond colored triangles.

use crate::game::Faction;
use crate::platform::Vertex;
use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

/// Radians per second.
const SPIN_SPEED: f32 = 0.45;
/// Lean of the rotation axis towards the viewer, in radians.
const AXIAL_TILT: f32 = 0.4;
/// Below this radius in pixels the coarser mesh looks the same and costs far less.
const DETAIL_RADIUS: f32 = 80.0;

struct Mesh {
    rows: usize,
    cols: usize,
    /// (rows + 1) * (cols + 1) points on the unit sphere, y up.
    points: Vec<[f32; 3]>,
    colors: Vec<[f32; 3]>,
    alphas: Vec<f32>,
}

/// The main menu's sandy planet turns very slowly, and its clouds a little faster around a
/// slightly different axis, so that they drift over the surface.
const SANDY_SPIN: f32 = 0.02;
const CLOUD_SPIN: f32 = 0.033;
const CLOUD_TILT: f32 = AXIAL_TILT + 0.13;
/// How far above the surface the clouds float, as a share of the radius.
const CLOUD_HEIGHT: f32 = 1.015;
const SANDY_SEED: u32 = 61;
const CLOUD_SEED: u32 = 83;
const SAND_LOW: [f32; 3] = [0.30, 0.20, 0.10];
const SAND_MIDDLE: [f32; 3] = [0.50, 0.37, 0.20];
const SAND_HIGH: [f32; 3] = [0.66, 0.54, 0.34];
const CLOUD_COLOR: [f32; 3] = [0.95, 0.93, 0.88];
/// How opaque clouds get at their thickest.
const CLOUD_ALPHA: f32 = 0.75;

/// Dark seas, shallows and land for each faction.
fn palette(faction: Faction) -> [[f32; 3]; 3] {
    match faction {
        Faction::A => [[0.06, 0.14, 0.42], [0.16, 0.36, 0.78], [0.62, 0.76, 0.95]],
        Faction::H => [[0.30, 0.06, 0.04], [0.70, 0.18, 0.10], [0.98, 0.62, 0.34]],
        Faction::O => [[0.03, 0.26, 0.30], [0.10, 0.58, 0.62], [0.70, 0.95, 0.90]],
    }
}

fn seed(faction: Faction) -> u32 {
    match faction {
        Faction::A => 11,
        Faction::H => 29,
        Faction::O => 47,
    }
}

/// Each planet starts at its own angle so they do not turn in step.
fn start_angle(faction: Faction) -> f32 {
    match faction {
        Faction::A => 0.0,
        Faction::H => 2.1,
        Faction::O => 4.2,
    }
}

fn lattice(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343)
        ^ (y as u32).wrapping_mul(0xD816_3841)
        ^ (z as u32).wrapping_mul(0xCB1A_B31F)
        ^ seed.wrapping_mul(0x9E37_79B1);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5BD1_E995);
    h ^= h >> 15;
    (h & 0xFFFF) as f32 / 65535.0
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Smooth 3D value noise from 0 to 1. Sampling in 3D means the surface has no seam.
fn value_noise(p: [f32; 3], seed: u32) -> f32 {
    let base = p.map(|v| v.floor());
    let [ix, iy, iz] = base.map(|v| v as i32);
    let [fx, fy, fz] = [0, 1, 2].map(|i| {
        let t = p[i] - base[i];
        t * t * (3.0 - 2.0 * t)
    });
    let at = |dx, dy, dz| lattice(ix + dx, iy + dy, iz + dz, seed);
    mix(
        mix(mix(at(0, 0, 0), at(1, 0, 0), fx), mix(at(0, 1, 0), at(1, 1, 0), fx), fy),
        mix(mix(at(0, 0, 1), at(1, 0, 1), fx), mix(at(0, 1, 1), at(1, 1, 1), fx), fy),
        fz,
    )
}

/// Several octaves of noise, normalized to 0..1.
pub fn fractal_noise(point: [f32; 3], seed: u32) -> f32 {
    let (mut sum, mut amplitude, mut frequency, mut total) = (0.0, 1.0, 2.2, 0.0);
    for octave in 0..5 {
        let scaled = point.map(|v| v * frequency + 37.0);
        sum += value_noise(scaled, seed.wrapping_add(octave)) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    sum / total
}

fn surface_color(point: [f32; 3], faction: Faction) -> [f32; 3] {
    /// Heights above this are land.
    const SHORE: f32 = 0.52;
    let [deep, shallow, land] = palette(faction);
    let height = fractal_noise(point, seed(faction));
    let (from, to, t) = if height < SHORE {
        (deep, shallow, (height - 0.25) / (SHORE - 0.25))
    } else {
        (shallow, land, (height - SHORE) / 0.08)
    };
    let t = t.clamp(0.0, 1.0);
    [mix(from[0], to[0], t), mix(from[1], to[1], t), mix(from[2], to[2], t)]
}

fn sandy_color(point: [f32; 3]) -> [f32; 3] {
    let height = fractal_noise(point, SANDY_SEED);
    let (from, to, t) = if height < 0.5 {
        (SAND_LOW, SAND_MIDDLE, (height - 0.25) / 0.25)
    } else {
        (SAND_MIDDLE, SAND_HIGH, (height - 0.5) / 0.2)
    };
    let t = t.clamp(0.0, 1.0);
    [mix(from[0], to[0], t), mix(from[1], to[1], t), mix(from[2], to[2], t)]
}

/// Clear where the noise is low, thickening smoothly where it is high.
fn cloud_alpha(point: [f32; 3]) -> f32 {
    let t = ((fractal_noise(point, CLOUD_SEED) - 0.5) / 0.15).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t) * CLOUD_ALPHA
}

fn build_mesh(faction: Faction, rows: usize, cols: usize) -> Mesh {
    build_sphere(rows, cols, |point| surface_color(point, faction), |_| 1.0)
}

fn build_sphere(
    rows: usize,
    cols: usize,
    color_at: impl Fn([f32; 3]) -> [f32; 3],
    alpha_at: impl Fn([f32; 3]) -> f32,
) -> Mesh {
    let mut points = Vec::with_capacity((rows + 1) * (cols + 1));
    let mut colors = Vec::with_capacity(points.capacity());
    let mut alphas = Vec::with_capacity(points.capacity());
    for row in 0..=rows {
        let latitude = PI / 2.0 - row as f32 / rows as f32 * PI;
        for col in 0..=cols {
            let longitude = col as f32 / cols as f32 * TAU;
            let point = [
                latitude.cos() * longitude.sin(),
                latitude.sin(),
                latitude.cos() * longitude.cos(),
            ];
            colors.push(color_at(point));
            alphas.push(alpha_at(point));
            points.push(point);
        }
    }
    Mesh { rows, cols, points, colors, alphas }
}

fn faction_index(faction: Faction) -> usize {
    match faction {
        Faction::A => 0,
        Faction::H => 1,
        Faction::O => 2,
    }
}

/// Meshes per faction: detailed, then coarse. Built once, on first use.
fn meshes() -> &'static [[Mesh; 2]; 3] {
    static MESHES: OnceLock<[[Mesh; 2]; 3]> = OnceLock::new();
    MESHES.get_or_init(|| {
        [Faction::A, Faction::H, Faction::O]
            .map(|faction| [build_mesh(faction, 48, 96), build_mesh(faction, 18, 36)])
    })
}

/// Appends the visible half of a planet, turned to where it is after `seconds`.
pub fn draw(out: &mut Vec<Vertex>, faction: Faction, cx: f32, cy: f32, radius: f32, seconds: f32) {
    let lod = usize::from(radius < DETAIL_RADIUS);
    let mesh = &meshes()[faction_index(faction)][lod];
    let spin = start_angle(faction) + seconds * SPIN_SPEED;
    draw_mesh(out, mesh, (cx, cy, radius), spin, AXIAL_TILT);
}

/// The sandy planet for the main menu, with its cloud layer over it.
pub fn draw_sandy(out: &mut Vec<Vertex>, cx: f32, cy: f32, radius: f32, seconds: f32) {
    static SURFACE: OnceLock<Mesh> = OnceLock::new();
    static CLOUDS: OnceLock<Mesh> = OnceLock::new();
    let surface = SURFACE.get_or_init(|| build_sphere(72, 144, sandy_color, |_| 1.0));
    let clouds = CLOUDS.get_or_init(|| build_sphere(72, 144, |_| CLOUD_COLOR, cloud_alpha));
    draw_mesh(out, surface, (cx, cy, radius), seconds * SANDY_SPIN, AXIAL_TILT);
    draw_mesh(out, clouds, (cx, cy, radius * CLOUD_HEIGHT), seconds * CLOUD_SPIN, CLOUD_TILT);
}

/// Appends the side of a sphere facing the viewer, skipping parts that are fully see-through.
fn draw_mesh(out: &mut Vec<Vertex>, mesh: &Mesh, (cx, cy, radius): (f32, f32, f32), spin: f32, tilt: f32) {
    let (spin_sin, spin_cos) = spin.sin_cos();
    let (tilt_sin, tilt_cos) = tilt.sin_cos();

    let turned: Vec<[f32; 3]> = mesh
        .points
        .iter()
        .map(|&[x, y, z]| {
            let (x, z) = (x * spin_cos + z * spin_sin, z * spin_cos - x * spin_sin);
            let (y, z) = (y * tilt_cos - z * tilt_sin, y * tilt_sin + z * tilt_cos);
            [cx + x * radius, cy - y * radius, z]
        })
        .collect();

    let stride = mesh.cols + 1;
    let vertex = |i: usize| {
        let [x, y, _] = turned[i];
        let [r, g, b] = mesh.colors[i];
        Vertex { x, y, r, g, b, a: mesh.alphas[i] }
    };
    for row in 0..mesh.rows {
        for col in 0..mesh.cols {
            let corners = [
                row * stride + col,
                row * stride + col + 1,
                (row + 1) * stride + col + 1,
                (row + 1) * stride + col,
            ];
            // Keeping quads that only just face away closes the disc at its rim.
            let facing: f32 = corners.iter().map(|&i| turned[i][2]).sum();
            if facing < -0.2 || corners.iter().all(|&i| mesh.alphas[i] == 0.0) {
                continue;
            }
            let [a, b, c, d] = corners.map(vertex);
            out.extend([a, b, c, a, c, d]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(faction: Faction, radius: f32, seconds: f32) -> Vec<Vertex> {
        let mut out = Vec::new();
        draw(&mut out, faction, 500.0, 400.0, radius, seconds);
        out
    }

    fn average_color(vertices: &[Vertex]) -> [f32; 3] {
        let n = vertices.len() as f32;
        [
            vertices.iter().map(|v| v.r).sum::<f32>() / n,
            vertices.iter().map(|v| v.g).sum::<f32>() / n,
            vertices.iter().map(|v| v.b).sum::<f32>() / n,
        ]
    }

    fn sandy(seconds: f32) -> Vec<Vertex> {
        let mut out = Vec::new();
        draw_sandy(&mut out, 500.0, 400.0, 400.0, seconds);
        out
    }

    #[test]
    fn the_sandy_planet_is_sandy_with_some_clear_sky_and_some_cloud() {
        let vertices = sandy(0.0);
        let (ground, clouds): (Vec<Vertex>, Vec<Vertex>) = vertices.iter().partition(|v| v.a == 1.0);
        assert!(!ground.is_empty() && !clouds.is_empty());
        let [r, g, b] = average_color(&ground);
        assert!(r > g && g > b, "sandy: {:?}", [r, g, b]);
        assert!(clouds.iter().all(|v| v.a <= CLOUD_ALPHA), "clouds stay see-through");
        assert!(clouds.iter().any(|v| v.a > 0.3), "but can be seen");
        assert!(clouds.len() < ground.len(), "with gaps between them");
    }

    #[test]
    fn the_sandy_planet_turns_very_slowly_and_its_clouds_a_bit_differently() {
        assert!(SANDY_SPIN < SPIN_SPEED / 10.0, "much slower than the faction planets");
        assert_ne!(CLOUD_SPIN, SANDY_SPIN);
        assert_ne!(CLOUD_TILT, AXIAL_TILT);
        assert_ne!(sandy(0.0), sandy(30.0));
        assert_eq!(sandy(12.0), sandy(12.0));

        let ground = |vertices: Vec<Vertex>| -> Vec<Vertex> { vertices.into_iter().filter(|v| v.a == 1.0).collect() };
        let early = ground(sandy(0.0))[0];
        let later = ground(sandy(1.0))[0];
        let moved = (early.x - later.x).hypot(early.y - later.y);
        assert!(moved < 400.0 * 0.05, "a tiny part of the way in a second: {moved}");
    }

    #[test]
    fn planets_stay_inside_their_circle_and_reach_its_rim() {
        for (faction, radius) in [(Faction::A, 200.0), (Faction::O, 40.0)] {
            let vertices = drawn(faction, radius, 1.3);
            assert_eq!(vertices.len() % 3, 0, "a triangle list");
            let farthest = vertices
                .iter()
                .map(|v| (v.x - 500.0).hypot(v.y - 400.0))
                .fold(0.0, f32::max);
            assert!(farthest <= radius + 0.01, "outside the circle: {farthest}");
            assert!(farthest >= radius * 0.98, "does not reach the rim: {farthest}");
        }
    }

    #[test]
    fn only_about_the_facing_half_is_drawn() {
        let mesh = &meshes()[0][0];
        let all_quads = mesh.rows * mesh.cols;
        let quads = drawn(Faction::A, 200.0, 0.0).len() / 6;
        assert!(quads > all_quads / 3 && quads < all_quads * 3 / 4, "{quads} of {all_quads}");
    }

    #[test]
    fn the_surface_moves_as_time_passes() {
        assert_ne!(drawn(Faction::H, 200.0, 0.0), drawn(Faction::H, 200.0, 1.0));
    }

    #[test]
    fn the_same_moment_always_looks_the_same() {
        assert_eq!(drawn(Faction::H, 200.0, 2.5), drawn(Faction::H, 200.0, 2.5));
    }

    #[test]
    fn small_planets_use_the_coarse_mesh() {
        assert!(drawn(Faction::A, 30.0, 0.0).len() * 4 < drawn(Faction::A, 200.0, 0.0).len());
    }

    #[test]
    fn surfaces_vary_and_have_both_sea_and_land() {
        for faction in [Faction::A, Faction::H, Faction::O] {
            let [deep, _, land] = palette(faction);
            let colors = &meshes()[faction_index(faction)][0].colors;
            let near = |c: &[f32; 3], target: [f32; 3]| {
                (c[0] - target[0]).abs() + (c[1] - target[1]).abs() + (c[2] - target[2]).abs() < 0.25
            };
            let sea = colors.iter().filter(|c| near(c, deep)).count();
            let ground = colors.iter().filter(|c| near(c, land)).count();
            let total = colors.len();
            assert!(sea > total / 50, "{faction:?}: too little sea, {sea} of {total}");
            assert!(ground > total / 50, "{faction:?}: too little land, {ground} of {total}");
        }
    }

    #[test]
    fn each_faction_reads_as_its_color() {
        let blue = average_color(&drawn(Faction::A, 200.0, 0.0));
        let red = average_color(&drawn(Faction::H, 200.0, 0.0));
        let cyan = average_color(&drawn(Faction::O, 200.0, 0.0));
        assert!(blue[2] > blue[0] && blue[2] > blue[1], "A is blue: {blue:?}");
        assert!(red[0] > red[1] && red[0] > red[2], "H is red: {red:?}");
        assert!(cyan[1] > cyan[0] && cyan[2] > cyan[0], "O is cyan: {cyan:?}");
    }

    #[test]
    fn the_surface_has_no_seam_where_longitude_wraps() {
        let mesh = &meshes()[1][0];
        let stride = mesh.cols + 1;
        for row in 0..=mesh.rows {
            let (first, last) = (mesh.colors[row * stride], mesh.colors[row * stride + mesh.cols]);
            for i in 0..3 {
                assert!((first[i] - last[i]).abs() < 0.02, "row {row}");
            }
        }
    }
}
