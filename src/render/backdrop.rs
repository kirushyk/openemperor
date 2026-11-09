//! The main menu's sky: spiral arms of glowing dust turning around the planet, motes drifting
//! away from it, and planes of an angular script rising slowly behind everything.

use super::{Canvas, Color};
use crate::platform::Vertex;
use std::f32::consts::{PI, TAU};

type Point = (f32, f32);

const ARMS: u32 = 3;
const PUFFS: u32 = 420;
const PUFF_SEGMENTS: usize = 10;
/// How much of a puff's center glow is left halfway to its rim.
const PUFF_MIDDLE: f32 = 0.35;
/// Radians per second the dust turns around the planet.
const DUST_SPIN: f32 = 0.008;
/// Radians an arm winds around the planet from its inner end to its outer end.
const ARM_WINDING: f32 = 2.6;
const DUST_INNER: [f32; 3] = [0.018, 0.05, 0.085];
const DUST_MIDDLE: [f32; 3] = [0.05, 0.024, 0.095];
const DUST_OUTER: [f32; 3] = [0.075, 0.02, 0.06];

const MOTES: u32 = 180;
/// Share of its way from the planet out to the end of its life a mote travels per second.
const MOTE_RATE: f32 = 0.018;
const MOTE_COLORS: [[f32; 3]; 3] = [[0.55, 0.85, 1.0], [1.0, 0.55, 0.8], [0.92, 0.92, 1.0]];

/// One plane of rising script. Lengths are shares of the screen height.
struct Plane {
    /// Half the height of a letter.
    size: f32,
    spacing: f32,
    /// How far it rises per second.
    speed: f32,
    alpha: f32,
    seed: u32,
}

/// From the farthest plane to the nearest: nearer letters are bigger, faster and clearer.
const PLANES: [Plane; 3] = [
    Plane {
        size: 0.013,
        spacing: 0.085,
        speed: 0.005,
        alpha: 0.13,
        seed: 11,
    },
    Plane {
        size: 0.023,
        spacing: 0.135,
        speed: 0.011,
        alpha: 0.2,
        seed: 23,
    },
    Plane {
        size: 0.04,
        spacing: 0.22,
        speed: 0.022,
        alpha: 0.3,
        seed: 37,
    },
];
/// Share of the places on a plane that hold a letter.
const OCCUPANCY: f32 = 0.6;
const STROKE_WIDTH: f32 = 0.085;
/// How far a sharp corner of a stroke may reach, as a multiple of half its width.
const MITER_LIMIT: f32 = 2.0;
/// Share of the screen height over which letters fade out at the top.
const TOP_FADE: f32 = 0.2;
/// How visible letters are in the middle of the screen, behind the menu.
const MIDDLE_ALPHA: f32 = 0.45;
/// Radians per second letters sway from side to side, and how far as a share of their size.
const SWAY_RATE: f32 = 0.14;
const SWAY: f32 = 0.5;

/// Strokes of each letter, straight between their corners, in a box from -1 to 1.
const LETTERS: &[&[&[Point]]] = &[
    &[
        &[(-0.7, -0.8), (0.5, -0.8), (-0.2, 0.0), (0.6, 0.8)],
        &[(-0.7, 0.2), (-0.7, 0.8), (-0.1, 0.8)],
    ],
    &[
        &[(-0.7, 0.8), (-0.7, -0.3), (0.0, -0.85), (0.7, -0.3)],
        &[(0.7, 0.1), (0.1, 0.8)],
    ],
    &[
        &[(0.7, -0.8), (-0.5, -0.8), (-0.5, 0.1), (0.2, 0.1)],
        &[(0.7, -0.2), (0.7, 0.8), (-0.6, 0.8)],
    ],
    &[
        &[(0.3, -0.9), (-0.5, 0.05), (0.5, -0.05), (-0.3, 0.9)],
        &[(0.55, -0.75), (0.8, -0.5)],
    ],
    &[
        &[(-0.8, 0.5), (-0.35, -0.8), (0.0, 0.0), (0.75, -0.35)],
        &[(0.3, 0.35), (0.3, 0.85)],
    ],
    &[
        &[(-0.2, -0.85), (-0.75, -0.3), (-0.2, 0.25)],
        &[(0.6, -0.25), (0.05, 0.3), (0.6, 0.85)],
    ],
    &[
        &[
            (-0.5, -0.85),
            (0.25, -0.85),
            (0.25, -0.1),
            (-0.35, 0.5),
            (-0.35, 0.85),
        ],
        &[(0.6, 0.2), (0.6, 0.85)],
    ],
    &[
        &[(-0.75, 0.85), (0.75, -0.85)],
        &[(-0.7, -0.75), (-0.1, -0.75), (-0.1, -0.25)],
        &[(0.1, 0.25), (0.1, 0.75), (0.7, 0.75)],
    ],
    &[
        &[(-0.8, -0.35), (-0.35, -0.8), (0.1, -0.35), (0.6, -0.8)],
        &[(-0.6, 0.35), (-0.1, 0.8), (0.35, 0.35), (0.8, 0.8)],
    ],
    &[
        &[
            (0.55, -0.85),
            (-0.55, -0.85),
            (-0.7, 0.0),
            (-0.55, 0.85),
            (0.55, 0.85),
        ],
        &[(0.0, -0.3), (0.5, 0.0), (0.0, 0.3)],
    ],
    &[
        &[(-0.65, 0.85), (-0.65, -0.45), (-0.2, -0.85), (-0.2, 0.4)],
        &[(0.25, 0.85), (0.25, -0.3), (0.75, -0.8)],
    ],
    &[&[
        (-0.7, -0.85),
        (0.65, -0.4),
        (-0.65, 0.05),
        (0.7, 0.45),
        (-0.3, 0.9),
    ]],
];

/// A repeatable number in 0..1 for this index.
fn noise(index: u32, salt: u32) -> f32 {
    let mut h = index.wrapping_mul(0x9E37_79B9) ^ salt.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 13;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// Draws the sky around the planet at `(cx, cy, radius)`, for a screen `width` by `height`.
pub(super) fn draw(
    canvas: &mut Canvas,
    (width, height): (f32, f32),
    planet: (f32, f32, f32),
    seconds: f32,
) {
    draw_dust(canvas, height, planet, seconds);
    draw_motes(canvas, height, planet, seconds);
    draw_script(canvas, (width, height), seconds);
}

fn draw_dust(canvas: &mut Canvas, height: f32, (cx, cy, radius): (f32, f32, f32), seconds: f32) {
    for i in 0..PUFFS {
        // Out along the arm, 0 by the planet and 1 at its end.
        let out = noise(i, 1);
        let wobble = (seconds * 0.05 + noise(i, 2) * TAU).sin() * 0.05;
        let angle = (i % ARMS) as f32 * TAU / ARMS as f32
            + out * ARM_WINDING
            + (noise(i, 3) - 0.5) * 0.7
            + wobble
            - seconds * DUST_SPIN;
        let distance = radius * (1.0 + 1.5 * out.powf(1.3));
        let center = (cx + distance * angle.cos(), cy + distance * angle.sin());
        let size = height * (0.1 + 0.17 * noise(i, 4));
        let color = if out < 0.5 {
            mix(DUST_INNER, DUST_MIDDLE, out * 2.0)
        } else {
            mix(DUST_MIDDLE, DUST_OUTER, out * 2.0 - 1.0)
        };
        let glow = 0.75 + 0.5 * noise(i, 5);
        puff(canvas, center, size, color.map(|c| c * glow));
    }
}

/// A soft round glow, brightest in the middle.
fn puff(canvas: &mut Canvas, center: Point, size: f32, color: [f32; 3]) {
    let ring = |i: usize, reach: f32| {
        let angle = i as f32 / PUFF_SEGMENTS as f32 * TAU;
        (
            center.0 + size * reach * angle.cos(),
            center.1 + size * reach * angle.sin(),
        )
    };
    let middle = color.map(|c| c * PUFF_MIDDLE);
    for i in 0..PUFF_SEGMENTS {
        let (a, b) = (ring(i, 0.5), ring(i + 1, 0.5));
        let (c, d) = (ring(i, 1.0), ring(i + 1, 1.0));
        canvas.vertices.extend([
            Vertex::light(center, color),
            Vertex::light(a, middle),
            Vertex::light(b, middle),
            Vertex::light(a, middle),
            Vertex::light(c, [0.0; 3]),
            Vertex::light(d, [0.0; 3]),
            Vertex::light(a, middle),
            Vertex::light(d, [0.0; 3]),
            Vertex::light(b, middle),
        ]);
    }
}

fn draw_motes(canvas: &mut Canvas, height: f32, (cx, cy, radius): (f32, f32, f32), seconds: f32) {
    for i in 0..MOTES {
        let life = (seconds * MOTE_RATE + noise(i, 6)).fract();
        let angle = noise(i, 7) * TAU - seconds * DUST_SPIN * 2.0;
        let distance = radius * (1.02 + 1.6 * life);
        let (x, y) = (cx + distance * angle.cos(), cy + distance * angle.sin());
        let twinkle = 0.6 + 0.4 * (seconds * (0.5 + noise(i, 8)) + noise(i, 9) * TAU).sin();
        let bright = (life * PI).sin() * twinkle;
        let color = MOTE_COLORS[i as usize % MOTE_COLORS.len()].map(|c| c * bright);
        let r = height * (0.001 + 0.0016 * noise(i, 10));
        let (left, top, right, bottom) = ((x - r, y), (x, y - r), (x + r, y), (x, y + r));
        canvas.vertices.extend(
            [left, top, right, left, right, bottom].map(|corner| Vertex::light(corner, color)),
        );
    }
}

fn draw_script(canvas: &mut Canvas, (width, height): (f32, f32), seconds: f32) {
    for plane in &PLANES {
        let spacing = plane.spacing * height;
        let size = plane.size * height;
        let margin = size * 1.5;
        let rows = ((height + margin * 2.0) / spacing).ceil() as u32 + 1;
        let cols = (width / spacing).ceil() as u32 + 1;
        // A whole number of rows, so that the top row wraps to the bottom without a jump.
        let span = rows as f32 * spacing;
        for row in 0..rows {
            for col in 0..cols {
                let id = row * 1009 + col * 31 + plane.seed * 65537;
                if noise(id, 11) > OCCUPANCY {
                    continue;
                }
                let start = (row as f32 + noise(id, 12) * 0.6) * spacing;
                let y = (start - seconds * plane.speed * height).rem_euclid(span) - margin;
                if y > height + margin {
                    continue;
                }
                let sway = (seconds * SWAY_RATE + noise(id, 13) * TAU).sin() * SWAY * size;
                let x = (col as f32 + noise(id, 14) * 0.6) * spacing + sway;
                let off_middle = ((x - width / 2.0).abs() / (width * 0.3)).min(1.0);
                let place = MIDDLE_ALPHA + (1.0 - MIDDLE_ALPHA) * off_middle;
                let top = ((y + margin) / (TOP_FADE * height)).clamp(0.0, 1.0);
                let gray = 0.72 + 0.28 * noise(id, 15);
                let color = [gray, gray, gray, plane.alpha * place * top];
                let which = (noise(id, 16) * LETTERS.len() as f32) as usize;
                for stroke in LETTERS[which] {
                    draw_stroke(canvas, stroke, (x, y), size, color);
                }
            }
        }
    }
}

/// A band along a line, joined at its corners so that see-through strokes do not overlap there.
fn draw_stroke(canvas: &mut Canvas, stroke: &[Point], (x, y): Point, size: f32, color: Color) {
    let half = (size * STROKE_WIDTH).max(0.75);
    let place = |(px, py): Point| (x + px * size, y + py * size);
    let direction = |(ax, ay): Point, (bx, by): Point| {
        let length = (bx - ax).hypot(by - ay);
        ((bx - ax) / length, (by - ay) / length)
    };
    let last = stroke.len() - 1;
    let edges = |i: usize| {
        let into = direction(stroke[i.max(1) - 1], stroke[i.max(1)]);
        let onward = direction(stroke[i.min(last - 1)], stroke[i.min(last - 1) + 1]);
        let (tx, ty) = direction((-into.0, -into.1), onward);
        let (nx, ny) = (-ty, tx);
        // Past a corner the band's edges meet further out than half its width.
        let reach = half / (nx * -into.1 + ny * into.0).max(1.0 / MITER_LIMIT);
        let (px, py) = place(stroke[i]);
        (
            Vertex::new((px + nx * reach, py + ny * reach), color),
            Vertex::new((px - nx * reach, py - ny * reach), color),
        )
    };
    for i in 0..stroke.len() - 1 {
        let ((a, b), (c, d)) = (edges(i), edges(i + 1));
        canvas.vertices.extend([a, c, d, a, d, b]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(seconds: f32) -> Vec<Vertex> {
        let mut canvas = Canvas::default();
        draw(
            &mut canvas,
            (1920.0, 1080.0),
            (960.0, 713.0, 518.0),
            seconds,
        );
        canvas.vertices
    }

    #[test]
    fn letters_stay_in_their_box_and_their_points_are_all_apart() {
        for strokes in LETTERS {
            for stroke in strokes.iter() {
                assert!(stroke.len() >= 2);
                assert!(stroke.iter().all(|p| p.0.abs() <= 1.0 && p.1.abs() <= 1.0));
                assert!(stroke.windows(2).all(|w| w[0] != w[1]));
            }
        }
    }

    #[test]
    fn a_right_angle_keeps_the_stroke_its_full_width_on_both_sides() {
        let mut canvas = Canvas::default();
        let corner = [(-1.0, 0.0), (0.0, 0.0), (0.0, 1.0)];
        draw_stroke(&mut canvas, &corner, (0.0, 0.0), 100.0, [1.0; 4]);
        let [a, b] = [canvas.vertices[1], canvas.vertices[2]];
        let half = 100.0 * STROKE_WIDTH;
        assert!(((a.x - b.x).hypot(a.y - b.y) - 2.0 * half * 2f32.sqrt()).abs() < 0.001);
    }

    #[test]
    fn nearer_planes_are_bigger_faster_and_clearer() {
        for pair in PLANES.windows(2) {
            assert!(pair[1].size > pair[0].size);
            assert!(pair[1].speed > pair[0].speed);
            assert!(pair[1].alpha > pair[0].alpha);
        }
    }

    #[test]
    fn the_sky_is_light_and_see_through_letters() {
        let vertices = drawn(5.0);
        let letters: Vec<&Vertex> = vertices.iter().filter(|v| v.a > 0.0).collect();
        assert!(!letters.is_empty());
        assert!(letters.len() < vertices.len(), "and glowing dust");
        assert!(letters.iter().all(|v| v.a <= PLANES[2].alpha));
    }

    #[test]
    fn the_letters_rise() {
        // Short enough that no letter wraps round from the top to the bottom.
        let (before, after) = (drawn(0.5), drawn(0.6));
        let moved = before
            .iter()
            .zip(&after)
            .filter(|(b, a)| b.a > 0.0 && a.a > 0.0)
            .filter(|(b, a)| a.y < b.y)
            .count();
        let total = before.iter().filter(|v| v.a > 0.0).count();
        assert!(moved > total / 2, "{moved} of {total}");
    }
}
