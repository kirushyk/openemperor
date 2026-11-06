//! The machines' models: bare metal with their owner's color on a few parts, built from bevelled
//! blocks, round tubes and bodies lofted from rings. Faces that neither the camera nor the sun
//! can see are left out as they are made.

use super::{Canvas, Point, cross, knee_of, unit_vector};
use crate::battlefield::SUN;
use crate::platform::WorldVertex;
use crate::units::{self, Role, Unit};
use std::f32::consts::{PI, TAU};

/// What a surface is made of: its color, and how much it shines, from 0 for none through paint
/// to 1 for bare polished metal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paint {
    pub color: [f32; 3],
    pub shine: f32,
}

const STEEL: Paint = Paint {
    color: [0.58, 0.6, 0.63],
    shine: 1.0,
};
const GUNMETAL: Paint = Paint {
    color: [0.27, 0.28, 0.3],
    shine: 0.85,
};
const BRONZE: Paint = Paint {
    color: [0.62, 0.45, 0.25],
    shine: 0.9,
};
const RUBBER: Paint = Paint {
    color: [0.07, 0.07, 0.07],
    shine: 0.0,
};
const GLASS: Paint = Paint {
    color: [0.06, 0.08, 0.11],
    shine: 0.0,
};
/// The dark inside of a tube or a nozzle.
const BORE: Paint = Paint {
    color: [0.03, 0.03, 0.03],
    shine: 0.0,
};
const LENS: Paint = Paint {
    color: [1.0, 0.98, 0.85],
    shine: 0.0,
};
const WHITE: Paint = Paint {
    color: [0.95, 0.95, 0.95],
    shine: 0.35,
};
const ROCKET: Paint = Paint {
    color: [0.9, 0.9, 0.85],
    shine: 0.35,
};
const BEACON: Paint = Paint {
    color: [1.0, 0.62, 0.1],
    shine: 0.0,
};
/// How much the owner's paint shines: a soft, wide sheen.
const PAINT_SHINE: f32 = 0.35;

/// How much of a model is worth making: all of it, or for one small on the screen, rounder parts
/// with fewer sides and no small fittings.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Detail {
    #[default]
    Full,
    Low,
}

/// What looks at the faces being made: the camera, from where it is, and whether its mirror
/// image under the water does too. The sun always does, for the shadows. With no camera, the
/// model only casts a shadow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Watchers {
    pub eye: [f32; 3],
    pub camera: bool,
    pub mirrored: bool,
}

impl Watchers {
    /// Whether any of them sees the face of a triangle with these corners and normals. A little
    /// past edge-on still counts, for faces whose corner normals bend around a round part.
    fn see(&self, points: &[[f32; 3]; 3], normals: &[[f32; 3]; 3]) -> bool {
        const SLACK: f32 = -0.05;
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let normal = unit_vector([0, 1, 2].map(|i| normals[0][i] + normals[1][i] + normals[2][i]));
        if dot(normal, SUN) > SLACK {
            return true;
        }
        let middle = [0, 1, 2].map(|i| (points[0][i] + points[1][i] + points[2][i]) / 3.0);
        let faces = |eye: [f32; 3]| {
            let to = unit_vector([0, 1, 2].map(|i| eye[i] - middle[i]));
            dot(normal, to) > SLACK
        };
        let [x, y, z] = self.eye;
        self.camera && (faces(self.eye) || self.mirrored && faces([x, -y, z]))
    }
}

/// Where a model stands and which way it faces: its origin in the world, and the world ways of
/// its own along (to its nose), up and across (to its right).
#[derive(Clone, Copy, Debug)]
pub struct Place {
    origin: [f32; 3],
    along: [f32; 3],
    up: [f32; 3],
    across: [f32; 3],
}

impl Place {
    /// Standing on the ground under the unit, turned to its heading.
    pub fn on_ground(unit: &Unit) -> Self {
        let (sin, cos) = unit.heading.sin_cos();
        Place {
            origin: [unit.x, unit.y, unit.z],
            along: [cos, 0.0, sin],
            up: [0.0, 1.0, 0.0],
            across: [-sin, 0.0, cos],
        }
    }

    /// Where the unit is up in the air, leaning with it into its turns.
    pub fn in_air(unit: &Unit) -> Self {
        let mut place = Place::on_ground(unit);
        place.origin[1] += unit.altitude;
        let (sin, cos) = unit.bank.sin_cos();
        let (right, up) = (place.across, place.up);
        place.across = [0, 1, 2].map(|i| right[i] * cos - up[i] * sin);
        place.up = [0, 1, 2].map(|i| right[i] * sin + up[i] * cos);
        place
    }

    /// A point given as [along, up, across] from the origin, in the world.
    pub fn point(&self, [a, u, c]: [f32; 3]) -> [f32; 3] {
        [0, 1, 2].map(|i| self.origin[i] + self.along[i] * a + self.up[i] * u + self.across[i] * c)
    }

    /// A way given as [along, up, across], in the world.
    pub fn way(&self, [a, u, c]: [f32; 3]) -> [f32; 3] {
        [0, 1, 2].map(|i| self.along[i] * a + self.up[i] * u + self.across[i] * c)
    }

    /// The same place with its origin moved by [along, up, across].
    fn moved(self, by: [f32; 3]) -> Self {
        Place {
            origin: self.point(by),
            ..self
        }
    }
}

/// A round part along a straight line: a barrel, a wheel, a pod or a leg.
#[derive(Clone, Copy, Debug)]
struct Tube {
    from: [f32; 3],
    to: [f32; 3],
    radii: (f32, f32),
    sides: usize,
    /// Smoothly round, or flat-sided like a turret.
    round: bool,
    /// Whether each end, the near one and the far one, is closed.
    ends: (bool, bool),
}

fn tube(from: [f32; 3], to: [f32; 3], radius: f32) -> Tube {
    Tube {
        from,
        to,
        radii: (radius, radius),
        sides: 10,
        round: true,
        ends: (true, true),
    }
}

impl Tube {
    fn taper(self, far: f32) -> Self {
        Tube {
            radii: (self.radii.0, far),
            ..self
        }
    }

    fn sides(self, sides: usize) -> Self {
        Tube { sides, ..self }
    }

    fn faceted(self) -> Self {
        Tube {
            round: false,
            ..self
        }
    }

    fn ends(self, near: bool, far: bool) -> Self {
        Tube {
            ends: (near, far),
            ..self
        }
    }
}

/// A body lofted from rings across it, from its tail to its nose: each [along, up, half height,
/// half width], with the top of it in another paint, if given.
struct Loft<'a> {
    rings: &'a [[f32; 4]],
    sides: usize,
    paint: Paint,
    top: Option<Paint>,
}

/// Makes a model's triangles, into the canvas's solid objects.
pub struct Model<'a> {
    out: &'a mut Vec<WorldVertex>,
    watchers: Option<Watchers>,
    fine: bool,
    place: Place,
}

impl<'a> Model<'a> {
    pub fn new(canvas: &'a mut Canvas, place: Place) -> Self {
        Model {
            out: &mut canvas.objects,
            watchers: canvas.watchers,
            fine: canvas.detail == Detail::Full,
            place,
        }
    }

    /// The same model, made from another place for a while.
    fn at(&mut self, place: Place) -> Model<'_> {
        Model {
            out: self.out,
            watchers: self.watchers,
            fine: self.fine,
            place,
        }
    }

    /// How many sides a round part has, fewer for a model made with less detail.
    fn sides(&self, sides: usize) -> usize {
        if self.fine {
            sides
        } else {
            (sides / 2).max(sides.min(6))
        }
    }

    fn triangle(&mut self, points: [[f32; 3]; 3], normals: [[f32; 3]; 3], paint: Paint) {
        let points = points.map(|p| self.place.point(p));
        let normals = normals.map(|n| self.place.way(n));
        if self.watchers.is_some_and(|w| !w.see(&points, &normals)) {
            return;
        }
        self.out.extend((0..3).map(|i| {
            let [x, y, z] = points[i];
            WorldVertex::shiny((x, y, z), normals[i], paint.color, paint.shine)
        }));
    }

    fn quad(&mut self, [a, b, c, d]: [[f32; 3]; 4], normal: [f32; 3], paint: Paint) {
        self.triangle([a, b, c], [normal; 3], paint);
        self.triangle([a, c, d], [normal; 3], paint);
    }

    /// A box with sharp edges, given by its middle and how far it reaches each way.
    fn block(&mut self, middle: [f32; 3], half: [f32; 3], paint: Paint) {
        self.bevel(middle, half, 0.0, paint);
    }

    /// A box with its edges and corners cut off by `cut`.
    fn bevel(&mut self, middle: [f32; 3], half: [f32; 3], cut: f32, paint: Paint) {
        let cut = cut.min(half[0]).min(half[1]).min(half[2]);
        let inner = half.map(|h| h - cut);
        // A point given by how far out it goes along each of the three ways.
        let at = |offset: [f32; 3]| [0, 1, 2].map(|i| middle[i] + offset[i]);
        let axis = |i: usize, value: f32| {
            let mut v = [0.0; 3];
            v[i] = value;
            v
        };
        for i in 0..3 {
            let (j, k) = ((i + 1) % 3, (i + 2) % 3);
            for s in [-1.0, 1.0] {
                // The face itself.
                let corner = |sj: f32, sk: f32| {
                    let mut p = [0.0; 3];
                    (p[i], p[j], p[k]) = (s * half[i], sj * inner[j], sk * inner[k]);
                    at(p)
                };
                let face = [
                    corner(-1.0, -1.0),
                    corner(1.0, -1.0),
                    corner(1.0, 1.0),
                    corner(-1.0, 1.0),
                ];
                self.quad(face, axis(i, s), paint);
                if cut <= 0.0 {
                    continue;
                }
                // The cut edge running along k, between this face and the one across j.
                for sj in [-1.0, 1.0] {
                    let point = |on_i: bool, sk: f32| {
                        let mut p = [0.0; 3];
                        p[i] = s * if on_i { half[i] } else { inner[i] };
                        p[j] = sj * if on_i { inner[j] } else { half[j] };
                        p[k] = sk * inner[k];
                        at(p)
                    };
                    let normal = unit_vector([0, 1, 2].map(|n| {
                        if n == i {
                            s
                        } else if n == j {
                            sj
                        } else {
                            0.0
                        }
                    }));
                    let strip = [
                        point(true, -1.0),
                        point(true, 1.0),
                        point(false, 1.0),
                        point(false, -1.0),
                    ];
                    self.quad(strip, normal, paint);
                }
            }
        }
        if cut <= 0.0 {
            return;
        }
        for signs in (0..8).map(|n| [0, 1, 2].map(|i| if n >> i & 1 == 1 { 1.0 } else { -1.0 })) {
            let corner = |out: usize| {
                at([0, 1, 2].map(|i| signs[i] * if i == out { half[i] } else { inner[i] }))
            };
            let normal = unit_vector(signs);
            self.triangle([corner(0), corner(1), corner(2)], [normal; 3], paint);
        }
    }

    /// A block of any eight corners: four around its foot and four around its head in the
    /// same order, for sloped and tapering parts.
    fn hexa(&mut self, corners: [[f32; 3]; 8], paint: Paint) {
        let middle = [0, 1, 2].map(|i| corners.iter().map(|c| c[i]).sum::<f32>() / 8.0);
        let mut face = |ids: [usize; 4]| {
            let [a, b, c, d] = ids.map(|n| corners[n]);
            let diagonals = (
                [0, 1, 2].map(|i| c[i] - a[i]),
                [0, 1, 2].map(|i| d[i] - b[i]),
            );
            let mut normal = unit_vector(cross(diagonals.0, diagonals.1));
            let out: f32 = (0..3)
                .map(|i| (a[i] + c[i] - 2.0 * middle[i]) * normal[i])
                .sum();
            if out < 0.0 {
                normal = normal.map(|n| -n);
            }
            self.quad([a, b, c, d], normal, paint);
        };
        face([0, 1, 2, 3]);
        face([4, 5, 6, 7]);
        for i in 0..4 {
            face([i, (i + 1) % 4, (i + 1) % 4 + 4, i + 4]);
        }
    }

    fn tube(&mut self, tube: Tube, paint: Paint) {
        let way: [f32; 3] = [0, 1, 2].map(|i| tube.to[i] - tube.from[i]);
        let length = (way[0] * way[0] + way[1] * way[1] + way[2] * way[2]).sqrt();
        if length <= 0.0 {
            return;
        }
        let w = way.map(|c| c / length);
        let other = if w[1].abs() < 0.9 {
            [0.0, 1.0, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        };
        let u = unit_vector(cross(w, other));
        let v = cross(w, u);
        let sides = self.sides(tube.sides);
        let turn = if tube.round { 0.0 } else { PI / sides as f32 };
        let out = |step: f32| {
            let (sin, cos) = (turn + step / sides as f32 * TAU).sin_cos();
            [0, 1, 2].map(|i| u[i] * cos + v[i] * sin)
        };
        let ring =
            |base: [f32; 3], radius: f32, o: [f32; 3]| [0, 1, 2].map(|i| base[i] + o[i] * radius);
        let (near, far) = tube.radii;
        let slope = (near - far) / length;
        let normal = |o: [f32; 3]| unit_vector([0, 1, 2].map(|i| o[i] + w[i] * slope));
        for side in 0..sides {
            let (o0, o1) = (out(side as f32), out(side as f32 + 1.0));
            let corners = [
                ring(tube.from, near, o0),
                ring(tube.from, near, o1),
                ring(tube.to, far, o1),
                ring(tube.to, far, o0),
            ];
            let [a, b, c, d] = corners;
            if tube.round {
                let (n0, n1) = (normal(o0), normal(o1));
                if near > 0.0 {
                    self.triangle([a, b, c], [n0, n1, n1], paint);
                }
                if far > 0.0 {
                    self.triangle([a, c, d], [n0, n1, n0], paint);
                }
            } else {
                self.quad(corners, normal(out(side as f32 + 0.5)), paint);
            }
            for (closed, base, radius, sign) in [
                (tube.ends.0, tube.from, near, -1.0),
                (tube.ends.1, tube.to, far, 1.0),
            ] {
                if closed && radius > 0.0 {
                    let cap = w.map(|c| c * sign);
                    let points = [base, ring(base, radius, o0), ring(base, radius, o1)];
                    self.triangle(points, [cap; 3], paint);
                }
            }
        }
    }

    fn loft(&mut self, loft: Loft) {
        let sides = self.sides(loft.sides);
        let angle = |step: f32| (step / sides as f32 * TAU).sin_cos();
        let point = |[along, up, high, wide]: [f32; 4], (sin, cos): (f32, f32)| {
            [along, up + high * sin, wide * cos]
        };
        let size = |ring: [f32; 4], (sin, cos): (f32, f32)| (ring[2] * sin).hypot(ring[3] * cos);
        let rings = loft.rings;
        let normal = |n: usize, turn: (f32, f32)| {
            let (sin, cos) = turn;
            let ring = rings[n];
            let flat = unit_vector([0.0, sin / ring[2].max(0.001), cos / ring[3].max(0.001)]);
            let (before, after) = (
                rings[n.saturating_sub(1)],
                rings[(n + 1).min(rings.len() - 1)],
            );
            let slope =
                (size(after, turn) - size(before, turn)) / (after[0] - before[0]).max(0.001);
            unit_vector([-slope, flat[1], flat[2]])
        };
        for n in 0..rings.len() - 1 {
            for side in 0..sides {
                let (t0, t1) = (angle(side as f32), angle(side as f32 + 1.0));
                let paint = match loft.top {
                    Some(top) if angle(side as f32 + 0.5).0 > 0.6 => top,
                    _ => loft.paint,
                };
                let (a, b) = (point(rings[n], t0), point(rings[n], t1));
                let (c, d) = (point(rings[n + 1], t1), point(rings[n + 1], t0));
                let (na, nb) = (normal(n, t0), normal(n, t1));
                let (nc, nd) = (normal(n + 1, t1), normal(n + 1, t0));
                self.triangle([a, b, c], [na, nb, nc], paint);
                self.triangle([a, c, d], [na, nc, nd], paint);
            }
        }
        // Close whichever end is not already a point.
        for (ring, sign) in [(rings[0], -1.0), (rings[rings.len() - 1], 1.0)] {
            if ring[2] > 0.0 && ring[3] > 0.0 {
                let middle = [ring[0], ring[1], 0.0];
                for side in 0..sides {
                    let (t0, t1) = (angle(side as f32), angle(side as f32 + 1.0));
                    let points = [middle, point(ring, t0), point(ring, t1)];
                    self.triangle(points, [[sign, 0.0, 0.0]; 3], loft.paint);
                }
            }
        }
    }

    /// A ball squashed to its three half sizes [along, up, across], or only its upper half.
    fn ball(&mut self, middle: [f32; 3], half: [f32; 3], upper: bool, paint: Paint) {
        let (rings, sides) = (self.sides(6), self.sides(10));
        let first = if upper { 0.0 } else { -PI / 2.0 };
        let at = |ring: usize, side: usize| {
            let lat = first + (PI / 2.0 - first) * ring as f32 / rings as f32;
            let lon = side as f32 / sides as f32 * TAU;
            let (lat_sin, lat_cos) = lat.sin_cos();
            let (lon_sin, lon_cos) = lon.sin_cos();
            let o = [lat_cos * lon_cos, lat_sin, lat_cos * lon_sin];
            let point = [0, 1, 2].map(|i| middle[i] + o[i] * half[i]);
            let normal = unit_vector([0, 1, 2].map(|i| o[i] / half[i].max(0.001)));
            (point, normal)
        };
        for ring in 0..rings {
            for side in 0..sides {
                let (a, na) = at(ring, side);
                let (b, nb) = at(ring, side + 1);
                let (c, nc) = at(ring + 1, side + 1);
                let (d, nd) = at(ring + 1, side);
                self.triangle([a, b, c], [na, nb, nc], paint);
                if ring + 1 < rings {
                    self.triangle([a, c, d], [na, nc, nd], paint);
                }
            }
        }
    }

    /// A thin square bar between two points.
    fn bar(&mut self, from: [f32; 3], to: [f32; 3], half: f32, paint: Paint) {
        let tube = tube(from, to, half * std::f32::consts::SQRT_2).sides(4);
        self.tube(tube.faceted(), paint);
    }

    /// A lamp: a glass lens in a steel housing, looking ahead along the model.
    fn lamp(&mut self, at: [f32; 3], radius: f32) {
        let behind = [at[0] - radius * 0.9, at[1], at[2]];
        let housing = tube(behind, at, radius * 1.25).taper(radius * 1.3).sides(8);
        self.tube(housing, GUNMETAL);
        let front = [at[0] + 0.006, at[1], at[2]];
        self.tube(tube(at, front, radius).sides(8).ends(false, true), LENS);
    }

    /// A round wheel across the model, its middle at `middle`, `width` wide, closed only on its
    /// outer side: its inner side faces the hull.
    fn wheel(&mut self, middle: [f32; 3], radius: f32, width: f32, sides: usize, paint: Paint) {
        let [a, u, c] = middle;
        let half = width / 2.0;
        let ends = (c <= 0.0, c >= 0.0);
        self.tube(
            tube([a, u, c - half], [a, u, c + half], radius)
                .sides(sides)
                .ends(ends.0, ends.1),
            paint,
        );
    }

    /// A track of separate links running around the wheels on one side, and the wheels in it.
    fn track(&mut self, track: &Track, across: f32, travel: f32) {
        let Track {
            rear,
            front,
            height,
            radius,
            half_width,
            road_wheels,
            links,
        } = *track;
        let links = if self.fine { links } else { links / 2 };
        const THICK: f32 = 0.022;
        // The loop runs along the top from the back, down around the front, back along the
        // bottom and up around the back.
        let run = front - rear;
        let around = PI * radius;
        let length = 2.0 * run + 2.0 * around;
        let spot = |s: f32| -> ([f32; 2], [f32; 2]) {
            let s = s.rem_euclid(length);
            if s < run {
                ([rear + s, height + radius], [1.0, 0.0])
            } else if s < run + around {
                let angle = PI / 2.0 - (s - run) / radius;
                let (sin, cos) = angle.sin_cos();
                ([front + radius * cos, height + radius * sin], [sin, -cos])
            } else if s < 2.0 * run + around {
                ([front - (s - run - around), height - radius], [-1.0, 0.0])
            } else {
                let angle = -PI / 2.0 - (s - 2.0 * run - around) / radius;
                let (sin, cos) = angle.sin_cos();
                ([rear + radius * cos, height + radius * sin], [sin, -cos])
            }
        };
        let gap = length / links as f32;
        let fine = self.fine;
        for link in 0..links {
            let (at, way) = spot(link as f32 * gap + travel);
            // Out from the loop, to the left of the way it runs.
            let out = [-way[1], way[0]];
            let half_long = gap * 0.42;
            let corner = |l: f32, o: f32, c: f32| {
                [
                    at[0] + way[0] * l + out[0] * o,
                    at[1] + way[1] * l + out[1] * o,
                    across + c,
                ]
            };
            let (way3, out3) = ([way[0], way[1], 0.0], [out[0], out[1], 0.0]);
            let back3 = way3.map(|v| -v);
            // A plate with no inside face: the wheels hide it. Its outside, its two edges and
            // its two ends.
            let mut plate = |(l0, l1): (f32, f32), (o0, o1): (f32, f32), c: f32, edges: bool| {
                let top = [
                    corner(l0, o1, -c),
                    corner(l1, o1, -c),
                    corner(l1, o1, c),
                    corner(l0, o1, c),
                ];
                self.quad(top, out3, GUNMETAL);
                for (l, normal) in [(l1, way3), (l0, back3)] {
                    let end = [
                        corner(l, o0, -c),
                        corner(l, o0, c),
                        corner(l, o1, c),
                        corner(l, o1, -c),
                    ];
                    self.quad(end, normal, GUNMETAL);
                }
                if edges {
                    for (cc, normal) in [(c, [0.0, 0.0, 1.0]), (-c, [0.0, 0.0, -1.0])] {
                        let edge = [
                            corner(l0, o0, cc),
                            corner(l1, o0, cc),
                            corner(l1, o1, cc),
                            corner(l0, o1, cc),
                        ];
                        self.quad(edge, normal, GUNMETAL);
                    }
                }
            };
            plate((-half_long, half_long), (0.0, THICK), half_width, true);
            if fine {
                // The grouser bar across the link's outside, for grip.
                let bar = (-half_long * 0.25, half_long * 0.25);
                plate(bar, (THICK, THICK + 0.012), half_width * 0.95, false);
            }
        }
        let wheel_radius = radius - 0.004;
        let width = half_width * 1.5;
        for n in 0..road_wheels {
            let along = rear + run * (n as f32 + 0.5) / road_wheels as f32;
            let middle = [along, height - radius + wheel_radius * 0.8, across];
            self.wheel(middle, wheel_radius * 0.8, width, 8, GUNMETAL);
        }
        // The drive sprocket at the back, its flats for teeth, and the round idler at the front.
        let half = width / 2.0;
        let ends = (across <= 0.0, across >= 0.0);
        let sprocket = tube(
            [rear, height, across - half],
            [rear, height, across + half],
            wheel_radius,
        )
        .sides(8)
        .faceted()
        .ends(ends.0, ends.1);
        self.tube(sprocket, STEEL);
        self.wheel(
            [front, height, across],
            wheel_radius * 0.92,
            width,
            10,
            GUNMETAL,
        );
        self.wheel(
            [front, height, across],
            wheel_radius * 0.4,
            width * 1.15,
            6,
            STEEL,
        );
    }
}

/// How a track runs: between its back and front wheel middles along the model, at a height,
/// around wheels of a radius, with so many road wheels and links in it.
#[derive(Clone, Copy, Debug)]
struct Track {
    rear: f32,
    front: f32,
    height: f32,
    radius: f32,
    half_width: f32,
    road_wheels: usize,
    links: usize,
}

/// How far each of a vehicle's tracks has run: the way it has gone, with the two sides running
/// against each other as it turns.
fn track_travel(unit: &Unit, across: f32) -> f32 {
    unit.stride - unit.heading * across
}

/// The owner's paint.
pub fn team(color: [f32; 3]) -> Paint {
    Paint {
        color,
        shine: PAINT_SHINE,
    }
}

/// Where a machine's lamps are, as [along, up, across] from where it is placed.
pub fn lamp_spots(role: Role) -> &'static [[f32; 3]] {
    match role {
        Role::Tank => &[[0.45, 0.2, -0.2], [0.45, 0.2, 0.2]],
        Role::RepairVehicle => &[[0.43, 0.24, 0.0]],
        Role::Catapult => &[[0.43, 0.17, 0.0]],
        Role::RocketLauncher => &[[0.46, 0.2, -0.15], [0.46, 0.2, 0.15]],
        Role::Strider => &[[0.25, 0.57, 0.0]],
        Role::HeavyStrider => &[[0.41, 1.15, -0.06], [0.41, 1.15, 0.06]],
        Role::AdvancedCarryall => &[[0.55, 0.03, -0.08], [0.55, 0.03, 0.08]],
        Role::AirDefensePlatform => &[[0.41, 0.05, -0.27], [0.41, 0.05, 0.27]],
        Role::Ornithopter => &[[0.23, 0.08, 0.0]],
        Role::Destroyer => &[[0.45, 0.09, -0.06], [0.45, 0.09, 0.06]],
        // The searchlight on a tower's roof.
        Role::Tower | Role::GunTower => &[[0.14, 1.5, 0.0]],
        _ => &[],
    }
}

/// How big a machine's lamp lenses are.
fn lamp_radius(role: Role) -> f32 {
    match role {
        Role::AdvancedCarryall => 0.045,
        Role::HeavyStrider => 0.035,
        Role::Tower | Role::GunTower => 0.045,
        _ => 0.026,
    }
}

/// A lamp's beam: where its lens is in the world and which way is across it, how wide the lens
/// is, how far ahead of it the beam reaches the ground and how wide it is there.
pub struct Beam {
    pub lens: Point,
    pub ahead: [f32; 3],
    pub right: [f32; 3],
    pub half: f32,
    pub reach: f32,
    pub spread: f32,
}

/// The beams a machine casts now: none from one sitting on the ground that only flies.
pub fn beams(unit: &Unit) -> Vec<Beam> {
    let spots = lamp_spots(unit.role);
    let flies_only = matches!(unit.role, Role::Ornithopter | Role::Destroyer);
    if spots.is_empty() || flies_only && unit.is_grounded() {
        return Vec::new();
    }
    let place = if unit.kind == units::UnitKind::Aircraft {
        Place::in_air(unit)
    } else {
        Place::on_ground(unit)
    };
    let (reach, spread) = match unit.role {
        Role::AdvancedCarryall => (8.0, 2.6),
        Role::AirDefensePlatform => (4.5, 0.8),
        Role::Ornithopter | Role::Destroyer => (5.0, 1.2),
        Role::HeavyStrider => (3.5, 1.0),
        // A searchlight throws a long, narrow beam far over the ground as the tower looks out.
        Role::Tower | Role::GunTower => (8.0, 0.9),
        _ if spots.len() == 1 => (2.6, 1.0),
        _ => (2.6, 0.75),
    };
    let (sin, cos) = unit.heading.sin_cos();
    spots
        .iter()
        .map(|&spot| {
            let [x, y, z] = place.point(spot);
            Beam {
                lens: (x, y, z),
                ahead: [cos, 0.0, sin],
                right: [-sin, 0.0, cos],
                half: lamp_radius(unit.role),
                reach,
                spread,
            }
        })
        .collect()
}

/// Makes a ground vehicle or an aircraft, in its owner's color where it is painted.
pub fn draw_machine(canvas: &mut Canvas, unit: &Unit, time: f32, color: [f32; 3]) {
    let team = team(color);
    let place = if unit.kind == units::UnitKind::Aircraft {
        Place::in_air(unit)
    } else {
        Place::on_ground(unit)
    };
    let model = &mut Model::new(canvas, place);
    match unit.role {
        Role::Tank => tank(model, unit, team),
        Role::RepairVehicle => repair_vehicle(model, unit, team),
        Role::Catapult => catapult(model, unit, team),
        Role::RocketLauncher => rocket_launcher(model, unit, team),
        Role::Strider => strider(model, unit, team),
        Role::HeavyStrider => heavy_strider(model, unit, team),
        Role::AdvancedCarryall => carryall(model, team),
        Role::AirDefensePlatform => air_defense_platform(model, team),
        Role::AirDrone => air_drone(model, team),
        Role::Ornithopter => ornithopter(model, unit, time, team),
        Role::Destroyer => destroyer(model, unit, time, team),
        _ => return,
    }
    let radius = lamp_radius(unit.role);
    for &spot in lamp_spots(unit.role) {
        model.lamp(spot, radius);
    }
}

/// Where the nozzle of an air drone is, as [along, up, across].
pub const DRONE_NOZZLE: [f32; 3] = [-0.31, 0.065, 0.0];

/// The linked tracks on both sides of a tracked vehicle.
fn tracks(model: &mut Model, unit: &Unit, track: &Track, apart: f32) {
    for across in [-apart, apart] {
        model.track(track, across, track_travel(unit, across));
    }
}

/// A squat battle tank: a hull with a sloped glacis on linked tracks, a turret of eight sides
/// with the owner's color on its roof, and a long gun with a muzzle brake.
fn tank(model: &mut Model, unit: &Unit, team: Paint) {
    let track = Track {
        rear: -0.35,
        front: 0.35,
        height: 0.1,
        radius: 0.085,
        half_width: 0.06,
        road_wheels: 5,
        links: 20,
    };
    tracks(model, unit, &track, 0.205);
    // The hull between the tracks, and above them the upper hull with its sloped front.
    model.bevel([0.0, 0.13, 0.0], [0.39, 0.07, 0.14], 0.02, GUNMETAL);
    let (low, high) = (0.2, 0.29);
    let upper = |a: f32, u: f32, c: f32| [a, u, c];
    model.hexa(
        [
            upper(-0.45, low, -0.28),
            upper(0.47, low, -0.28),
            upper(0.47, low, 0.28),
            upper(-0.45, low, 0.28),
            upper(-0.43, high, -0.27),
            upper(0.3, high, -0.27),
            upper(0.3, high, 0.27),
            upper(-0.43, high, 0.27),
        ],
        STEEL,
    );
    // The skirts over the top run of the tracks, and the owner's band above them.
    for side in [-1.0, 1.0] {
        model.block([0.0, 0.205, side * 0.285], [0.42, 0.035, 0.006], STEEL);
        model.block([-0.05, 0.255, side * 0.276], [0.34, 0.018, 0.006], team);
    }
    // The grille slats on the engine deck.
    if model.fine {
        for along in [-0.38, -0.33, -0.28] {
            model.block([along, high + 0.006, 0.0], [0.012, 0.006, 0.17], GUNMETAL);
        }
    }
    // The turret, its roof, the hatch, the mantlet and the gun.
    let (turret_foot, turret_top) = (high, 0.43);
    let turret = tube([-0.05, turret_foot, 0.0], [-0.05, turret_top, 0.0], 0.19)
        .taper(0.15)
        .sides(8)
        .faceted();
    model.tube(turret, STEEL);
    let roof = tube(
        [-0.05, turret_top, 0.0],
        [-0.05, turret_top + 0.008, 0.0],
        0.14,
    )
    .sides(8)
    .faceted();
    model.tube(roof, team);
    if model.fine {
        let hatch = [-0.11, turret_top + 0.008, 0.06];
        model.tube(
            tube(hatch, [hatch[0], hatch[1] + 0.025, hatch[2]], 0.045).sides(8),
            GUNMETAL,
        );
        model.tube(
            tube([-0.4, high, -0.2], [-0.4, 0.66, -0.2], 0.006).sides(4),
            GUNMETAL,
        );
    }
    let gun = 0.375;
    model.bevel([0.13, gun, 0.0], [0.05, 0.05, 0.075], 0.015, GUNMETAL);
    model.tube(tube([0.17, gun, 0.0], [0.58, gun, 0.0], 0.026), GUNMETAL);
    model.tube(tube([0.58, gun, 0.0], [0.65, gun, 0.0], 0.038), GUNMETAL);
}

/// A tracked workshop: a cab with a windscreen and a beacon, a taller workshop behind it with
/// a folded crane and a white cross on the roof.
fn repair_vehicle(model: &mut Model, unit: &Unit, team: Paint) {
    let track = Track {
        rear: -0.32,
        front: 0.32,
        height: 0.085,
        radius: 0.075,
        half_width: 0.06,
        road_wheels: 4,
        links: 18,
    };
    tracks(model, unit, &track, 0.19);
    model.bevel([0.0, 0.13, 0.0], [0.37, 0.06, 0.13], 0.02, GUNMETAL);
    model.block([0.0, 0.185, 0.0], [0.42, 0.012, 0.26], STEEL);
    // The cab, painted, with its sloped windscreen.
    let (foot, top) = (0.19, 0.35);
    model.hexa(
        [
            [0.18, foot, -0.25],
            [0.42, foot, -0.25],
            [0.42, foot, 0.25],
            [0.18, foot, 0.25],
            [0.18, top, -0.24],
            [0.33, top, -0.24],
            [0.33, top, 0.24],
            [0.18, top, 0.24],
        ],
        team,
    );
    let slope = unit_vector([0.16, 0.09, 0.0]);
    let screen = |t: f32, c: f32| [0.42 - 0.09 * t + 0.004, foot + 0.16 * t, c];
    model.quad(
        [
            screen(0.45, -0.2),
            screen(0.45, 0.2),
            screen(0.92, 0.2),
            screen(0.92, -0.2),
        ],
        slope,
        GLASS,
    );
    model.tube(
        tube([0.25, top, 0.0], [0.25, top + 0.045, 0.0], 0.025).sides(8),
        BEACON,
    );
    // The workshop, its panel seams and the owner's stripe along it.
    let (shop_foot, shop_top) = (0.19, 0.43);
    let shop_middle = (shop_foot + shop_top) / 2.0;
    let shop_half = (shop_top - shop_foot) / 2.0;
    model.bevel(
        [-0.12, shop_middle, 0.0],
        [0.29, shop_half, 0.25],
        0.025,
        STEEL,
    );
    for side in [-1.0, 1.0] {
        model.block([-0.12, 0.34, side * 0.252], [0.27, 0.022, 0.004], team);
        if model.fine {
            for along in [-0.3, -0.12, 0.06] {
                model.block(
                    [along, shop_middle, side * 0.253],
                    [0.005, shop_half * 0.85, 0.003],
                    GUNMETAL,
                );
            }
        }
    }
    // The white cross on the roof.
    model.block([-0.12, shop_top + 0.008, 0.0], [0.17, 0.008, 0.05], WHITE);
    model.block([-0.12, shop_top + 0.008, 0.0], [0.05, 0.008, 0.17], WHITE);
    // The crane, folded down on the roof's back corner.
    let base = [-0.33, shop_top, 0.17];
    let pivot = [-0.33, shop_top + 0.06, 0.17];
    let elbow = [0.0, shop_top + 0.08, 0.17];
    let tip = [-0.18, shop_top + 0.06, 0.17];
    model.tube(tube(base, pivot, 0.03).sides(8), GUNMETAL);
    model.bar(pivot, elbow, 0.014, STEEL);
    model.bar(elbow, tip, 0.012, STEEL);
    if model.fine {
        model.ball(elbow, [0.022; 3], false, BRONZE);
        model.ball(pivot, [0.022; 3], false, BRONZE);
        let hook = [tip[0], tip[1] - 0.03, tip[2]];
        model.bar(tip, hook, 0.005, GUNMETAL);
    }
}

/// A tracked carrier with an open hull, painted sides, and a long throwing arm on a pivot
/// with a bucket behind and a painted counterweight in front.
fn catapult(model: &mut Model, unit: &Unit, team: Paint) {
    let track = Track {
        rear: -0.32,
        front: 0.32,
        height: 0.085,
        radius: 0.075,
        half_width: 0.06,
        road_wheels: 4,
        links: 18,
    };
    tracks(model, unit, &track, 0.19);
    model.bevel([0.0, 0.12, 0.0], [0.37, 0.06, 0.13], 0.02, GUNMETAL);
    model.block([0.0, 0.18, 0.0], [0.41, 0.008, 0.25], GUNMETAL);
    // The open hull's walls: painted sides, steel ends.
    for side in [-1.0, 1.0] {
        model.bevel([0.0, 0.225, side * 0.25], [0.41, 0.045, 0.012], 0.006, team);
        model.bevel([side * 0.4, 0.225, 0.0], [0.012, 0.045, 0.24], 0.006, STEEL);
    }
    // The pivot and its axle.
    let pivot = [-0.02, 0.33, 0.0];
    model.bevel([-0.02, 0.26, 0.0], [0.06, 0.07, 0.08], 0.015, GUNMETAL);
    model.tube(
        tube([pivot[0], pivot[1], -0.1], [pivot[0], pivot[1], 0.1], 0.024).sides(8),
        BRONZE,
    );
    // The arm: two beams braced together, from the counterweight to the bucket.
    let (front, back) = ([0.2, 0.31, 0.0], [-0.4, 0.38, 0.0]);
    let at = |t: f32, c: f32| {
        [
            front[0] + (back[0] - front[0]) * t,
            front[1] + (back[1] - front[1]) * t,
            c,
        ]
    };
    for c in [-0.04, 0.04] {
        model.bar(at(0.0, c), at(1.0, c), 0.016, STEEL);
    }
    for t in [0.15, 0.55, 0.85] {
        model.bar(at(t, -0.04), at(t, 0.04), 0.01, STEEL);
    }
    // The bucket, a round cup open at its top.
    let cup = [-0.42, 0.36, 0.0];
    model.tube(
        tube(cup, [cup[0], cup[1] + 0.1, cup[2]], 0.045)
            .taper(0.09)
            .sides(10)
            .ends(true, false),
        GUNMETAL,
    );
    model.tube(
        tube(
            [cup[0], cup[1] + 0.03, cup[2]],
            [cup[0], cup[1] + 0.098, cup[2]],
            0.06,
        )
        .taper(0.084)
        .sides(10)
        .ends(true, false),
        BORE,
    );
    model.bevel([0.22, 0.33, 0.0], [0.07, 0.06, 0.09], 0.015, team);
}

/// A six-wheeled truck with an armored cab and two open racks of rockets that empty as the
/// salvo goes.
fn rocket_launcher(model: &mut Model, unit: &Unit, team: Paint) {
    for along in [-0.28, 0.0, 0.28] {
        for side in [-1.0, 1.0] {
            model.wheel([along, 0.085, side * 0.215], 0.085, 0.07, 12, RUBBER);
            model.wheel([along, 0.085, side * 0.215], 0.045, 0.078, 8, STEEL);
        }
    }
    model.block([0.0, 0.14, 0.0], [0.42, 0.035, 0.16], GUNMETAL);
    for side in [-1.0, 1.0] {
        // Mudguards over the wheels.
        model.block([0.0, 0.185, side * 0.215], [0.4, 0.008, 0.05], GUNMETAL);
    }
    // The cab, painted, with its windscreen, and the bumper with the lamps.
    let (foot, top) = (0.17, 0.34);
    model.hexa(
        [
            [0.24, foot, -0.2],
            [0.44, foot, -0.2],
            [0.44, foot, 0.2],
            [0.24, foot, 0.2],
            [0.24, top, -0.19],
            [0.37, top, -0.19],
            [0.37, top, 0.19],
            [0.24, top, 0.19],
        ],
        team,
    );
    let slope = unit_vector([0.17, 0.07, 0.0]);
    let screen = |t: f32, c: f32| [0.44 - 0.07 * t + 0.004, foot + 0.17 * t, c];
    model.quad(
        [
            screen(0.5, -0.16),
            screen(0.5, 0.16),
            screen(0.93, 0.16),
            screen(0.93, -0.16),
        ],
        slope,
        GLASS,
    );
    model.block([0.445, 0.18, 0.0], [0.015, 0.025, 0.22], GUNMETAL);
    // The bed and the racks on it.
    model.block([-0.1, 0.19, 0.0], [0.3, 0.02, 0.24], STEEL);
    let (rail_low, rail_high) = (0.31, 0.43);
    for rack in [-0.14, 0.14] {
        let side: f32 = if rack < 0.0 { -1.0 } else { 1.0 };
        model.block([-0.04, 0.26, rack], [0.06, 0.05, 0.05], GUNMETAL);
        for (u, c) in [
            (rail_low, rack - 0.1),
            (rail_low, rack + 0.1),
            (rail_high, rack - 0.1),
            (rail_high, rack + 0.1),
        ] {
            model.bar([-0.3, u, c], [0.3, u, c], 0.009, STEEL);
        }
        for along in [-0.28, 0.0, 0.28] {
            for c in [rack - 0.1, rack + 0.1] {
                model.bar([along, rail_low, c], [along, rail_high, c], 0.008, STEEL);
            }
            model.bar(
                [along, rail_low, rack - 0.1],
                [along, rail_low, rack + 0.1],
                0.008,
                STEEL,
            );
        }
        // The painted plate on its outer side.
        model.block(
            [0.0, (rail_low + rail_high) / 2.0, rack + side * 0.104],
            [0.29, 0.055, 0.004],
            team,
        );
    }
    let height = units::ROCKET_HEIGHT + 0.02;
    for &side in &units::ROCKET_SIDES[unit.salvo_fired..] {
        model.tube(
            tube([-0.28, height, side], [0.2, height, side], 0.02).sides(6),
            ROCKET,
        );
        model.tube(
            tube([0.2, height, side], [0.28, height, side], 0.02)
                .taper(0.0)
                .sides(6),
            ROCKET,
        );
        if model.fine {
            for fin in [[0.0, 0.03], [0.0, -0.03], [0.03, 0.0], [-0.03, 0.0]] {
                let tip = [-0.24, height + fin[0], side + fin[1]];
                model.bar([-0.28, height, side], tip, 0.004, GUNMETAL);
            }
        }
    }
}

/// How a striding unit's legs are built: how far apart, how long, and how far they swing.
struct Legs {
    side: f32,
    thigh: f32,
    shin: f32,
    swing: f32,
    /// How many radians of its step to each world unit it walks.
    pace: f32,
    width: f32,
    /// The paint of armored caps on the knees, if it has them.
    knee_caps: Option<Paint>,
    pistons: usize,
}

/// Two legs with knees under a striding unit, stepping in turn as it walks: round thighs and
/// shins on ball joints, pistons beside them, and three toes on each foot.
fn legs(model: &mut Model, unit: &Unit, legs: &Legs) {
    let hip_height = (legs.thigh + legs.shin) * 0.92;
    let moving = unit.is_moving();
    let scale = legs.thigh / 0.3;
    for (side, step) in [(-legs.side, 0.0), (legs.side, PI)] {
        let (swing, lift) = if moving {
            let phase = unit.stride * legs.pace + step;
            (
                legs.swing * phase.sin(),
                legs.swing * 0.5 * phase.cos().max(0.0),
            )
        } else {
            (0.0, 0.0)
        };
        let knee = knee_of((0.0, hip_height), (swing, lift), legs.thigh, legs.shin);
        let hip = [0.0, hip_height, side];
        let knee = [knee.0, knee.1, side];
        let foot = [swing, lift + 0.02 * scale, side];
        let half = legs.width / 2.0;
        model.ball(hip, [half * 1.5; 3], false, BRONZE);
        model.tube(tube(hip, knee, half).taper(half * 0.85).sides(8), STEEL);
        model.ball(knee, [half * 1.25; 3], false, BRONZE);
        if let Some(cap) = legs.knee_caps {
            let front = [knee[0] + half * 0.9, knee[1], knee[2]];
            model.ball(front, [half * 0.7, half * 1.4, half * 1.3], false, cap);
        }
        model.tube(
            tube(knee, foot, half * 0.75).taper(half * 0.6).sides(8),
            GUNMETAL,
        );
        if model.fine {
            let between =
                |a: [f32; 3], b: [f32; 3], t: f32| [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t);
            for n in 0..legs.pistons {
                let out = if n == 0 { 1.0 } else { -1.0 } * half * 1.1;
                let mut from = between(hip, knee, 0.55);
                let mut to = between(knee, foot, 0.6);
                from[2] += out;
                to[2] += out;
                model.tube(tube(from, to, half * 0.28).sides(6), STEEL);
            }
        }
        // Three toes ahead and a heel behind, flat on the ground.
        let toe = 0.1 * scale;
        for (turn, length) in [(-0.5, toe), (0.0, toe * 1.2), (0.5, toe), (PI, toe * 0.6)] {
            let (sin, cos) = turn.sin_cos();
            let tip = [
                foot[0] + cos * length,
                lift + 0.008 * scale,
                side + sin * length,
            ];
            model.bar(foot, tip, 0.016 * scale, GUNMETAL);
        }
    }
}

/// A small walking missile carrier: a rounded cockpit with a visor up on two bird legs, and a
/// pod of four missile tubes on each side.
fn strider(model: &mut Model, unit: &Unit, team: Paint) {
    legs(
        model,
        unit,
        &Legs {
            side: 0.15,
            thigh: 0.3,
            shin: 0.28,
            swing: 0.22,
            pace: 5.0,
            width: 0.07,
            knee_caps: None,
            pistons: 1,
        },
    );
    model.loft(Loft {
        rings: &[
            [-0.23, 0.62, 0.0, 0.0],
            [-0.21, 0.62, 0.07, 0.09],
            [-0.1, 0.62, 0.12, 0.14],
            [0.08, 0.62, 0.12, 0.15],
            [0.2, 0.61, 0.09, 0.12],
            [0.25, 0.6, 0.04, 0.06],
            [0.26, 0.6, 0.0, 0.0],
        ],
        sides: 12,
        paint: STEEL,
        top: Some(team),
    });
    model.block([0.205, 0.64, 0.0], [0.03, 0.022, 0.085], GLASS);
    let mouth_height = 0.66;
    for side in [-0.21, 0.21] {
        let out: f32 = if side < 0.0 { -1.0 } else { 1.0 };
        model.bevel(
            [0.0, mouth_height, side],
            [0.16, 0.05, 0.05],
            0.012,
            GUNMETAL,
        );
        model.block(
            [0.0, mouth_height, side + out * 0.051],
            [0.13, 0.035, 0.003],
            team,
        );
        for (u, c) in [
            (-0.022, -0.022),
            (-0.022, 0.022),
            (0.022, -0.022),
            (0.022, 0.022),
        ] {
            let at = [0.16, mouth_height + u, side + c];
            model.tube(
                tube(at, [0.163, at[1], at[2]], 0.015)
                    .sides(6)
                    .ends(false, true),
                BORE,
            );
        }
    }
}

/// A walking gun platform: a broad body on two big legs with armored knees, a raised painted
/// top, a sensor head with two lamps, four cannons that slide back as they fire, and exhausts.
fn heavy_strider(model: &mut Model, unit: &Unit, team: Paint) {
    const RECOIL_TRAVEL: f32 = 0.25;
    const BARREL_HALF: f32 = 0.28;
    legs(
        model,
        unit,
        &Legs {
            side: 0.3,
            thigh: 0.48,
            shin: 0.46,
            swing: 0.34,
            pace: 3.2,
            width: 0.12,
            knee_caps: Some(team),
            pistons: 2,
        },
    );
    model.bevel([0.0, 0.92, 0.0], [0.42, 0.1, 0.36], 0.04, STEEL);
    model.hexa(
        [
            [-0.28, 1.02, -0.24],
            [0.28, 1.02, -0.24],
            [0.28, 1.02, 0.24],
            [-0.28, 1.02, 0.24],
            [-0.2, 1.24, -0.16],
            [0.18, 1.24, -0.16],
            [0.18, 1.24, 0.16],
            [-0.2, 1.24, 0.16],
        ],
        team,
    );
    model.bevel([0.32, 1.15, 0.0], [0.08, 0.06, 0.12], 0.02, GUNMETAL);
    model.block(
        [0.43, units::CANNON_HEIGHT + 0.03, 0.0],
        [0.035, 0.07, 0.38],
        GUNMETAL,
    );
    let height = units::CANNON_HEIGHT + 0.03;
    for (&side, &recoil) in units::CANNON_SIDES.iter().zip(&unit.recoil) {
        let middle = units::CANNON_REACH - BARREL_HALF - RECOIL_TRAVEL * recoil;
        let (back, front) = (middle - BARREL_HALF, middle + BARREL_HALF);
        model.tube(
            tube([back, height, side], [front, height, side], 0.03),
            GUNMETAL,
        );
        model.tube(
            tube([front - 0.05, height, side], [front, height, side], 0.042),
            GUNMETAL,
        );
        model.tube(
            tube([0.43, height, side], [0.56, height, side], 0.046),
            STEEL,
        );
    }
    for side in [-0.2, 0.2] {
        model.tube(
            tube([-0.38, 1.0, side], [-0.38, 1.3, side], 0.035)
                .sides(8)
                .ends(false, false),
            GUNMETAL,
        );
        model.tube(
            tube([-0.38, 1.26, side], [-0.38, 1.29, side], 0.028).sides(8),
            BORE,
        );
    }
}

/// A long lifting frame: a lattice spine with a painted band, an engine nacelle on a pylon at
/// each side, a clamp frame underneath and two floodlights under its nose.
fn carryall(model: &mut Model, team: Paint) {
    let ribs = [-0.45, -0.27, -0.09, 0.09, 0.27, 0.45];
    let (low, high, apart) = (0.08, 0.17, 0.1);
    for c in [-apart, apart] {
        model.bar([-0.5, low, c], [0.5, low, c], 0.022, STEEL);
    }
    model.bar([-0.5, high, 0.0], [0.5, high, 0.0], 0.022, STEEL);
    for along in ribs {
        model.bar([along, low, -apart], [along, low, apart], 0.012, STEEL);
        for c in [-apart, apart] {
            model.bar([along, low, c], [along, high, 0.0], 0.012, STEEL);
        }
    }
    model.bevel([0.0, 0.125, 0.0], [0.06, 0.06, 0.125], 0.015, team);
    model.bevel([0.52, 0.11, 0.0], [0.05, 0.05, 0.13], 0.02, STEEL);
    model.bevel([-0.52, 0.11, 0.0], [0.03, 0.045, 0.12], 0.015, STEEL);
    for side in [-0.42, 0.42] {
        let out: f32 = if side < 0.0 { -1.0 } else { 1.0 };
        let height = 0.1;
        model.block([0.0, height, out * 0.24], [0.06, 0.014, 0.14], STEEL);
        model.tube(
            tube([-0.2, height, side], [0.16, height, side], 0.075).sides(12),
            STEEL,
        );
        model.tube(
            tube([0.16, height, side], [0.21, height, side], 0.083)
                .sides(12)
                .ends(false, false),
            team,
        );
        model.tube(
            tube([0.15, height, side], [0.2, height, side], 0.06)
                .sides(10)
                .ends(false, true),
            BORE,
        );
        model.tube(
            tube([-0.2, height, side], [-0.26, height, side], 0.065)
                .taper(0.05)
                .sides(10)
                .ends(false, false),
            GUNMETAL,
        );
        model.tube(
            tube([-0.21, height, side], [-0.25, height, side], 0.045)
                .sides(8)
                .ends(false, true),
            BORE,
        );
    }
    // The clamps the cables hang from.
    for along in [-0.15, 0.15] {
        for c in [-0.15, 0.15] {
            model.block([along, 0.03, c], [0.03, 0.04, 0.02], GUNMETAL);
        }
    }
}

/// A gun platform hovering on two capsule floats, painted on top, with a girder bridge between
/// them, a painted turret on a ring mount and a twin automatic gun.
fn air_defense_platform(model: &mut Model, team: Paint) {
    for side in [-0.27, 0.27] {
        let place = model.place.moved([0.0, 0.0, side]);
        model.at(place).loft(Loft {
            rings: &[
                [-0.4, 0.05, 0.0, 0.0],
                [-0.38, 0.05, 0.03, 0.06],
                [-0.32, 0.05, 0.05, 0.095],
                [0.3, 0.05, 0.05, 0.095],
                [0.36, 0.05, 0.035, 0.07],
                [0.4, 0.05, 0.0, 0.0],
            ],
            sides: 12,
            paint: STEEL,
            top: Some(team),
        });
    }
    // The girder bridge between the floats.
    for along in [-0.07, 0.07] {
        for height in [0.03, 0.09] {
            model.bar([along, height, -0.2], [along, height, 0.2], 0.012, STEEL);
        }
        for c in [-0.2, 0.0, 0.2] {
            model.bar([along, 0.03, c], [along, 0.09, c], 0.009, STEEL);
        }
    }
    model.tube(
        tube([0.0, 0.08, 0.0], [0.0, 0.11, 0.0], 0.12).sides(14),
        GUNMETAL,
    );
    let turret = tube([0.0, 0.11, 0.0], [0.0, 0.25, 0.0], 0.1)
        .taper(0.08)
        .sides(8)
        .faceted();
    model.tube(turret, team);
    // The twin gun: a receiver, two barrels and the ammunition box beside them.
    let gun = 0.175;
    model.bevel([0.1, gun, 0.0], [0.06, 0.03, 0.05], 0.01, GUNMETAL);
    for c in [-0.022, 0.022] {
        model.tube(
            tube([0.14, gun, c], [0.5, gun, c], 0.013).sides(8),
            GUNMETAL,
        );
    }
    model.bevel([0.04, gun, 0.11], [0.05, 0.04, 0.03], 0.008, GUNMETAL);
}

/// A small fast delta-wing drone: a slim fuselage with a sensor dome, a delta wing with
/// painted tips, a painted fin, and a round engine nozzle at its tail.
fn air_drone(model: &mut Model, team: Paint) {
    model.loft(Loft {
        rings: &[
            [-0.28, 0.065, 0.035, 0.035],
            [-0.2, 0.07, 0.045, 0.05],
            [0.1, 0.08, 0.05, 0.06],
            [0.38, 0.07, 0.03, 0.035],
            [0.5, 0.06, 0.0, 0.0],
        ],
        sides: 10,
        paint: STEEL,
        top: None,
    });
    let [a, u, c] = DRONE_NOZZLE;
    model.tube(
        tube([-0.28, u, c], [a, u, c], 0.036)
            .taper(0.03)
            .sides(10)
            .ends(false, false),
        GUNMETAL,
    );
    model.tube(
        tube([-0.27, u, c], [a + 0.002, u, c], 0.026)
            .sides(8)
            .ends(false, true),
        BORE,
    );
    model.ball([0.3, 0.095, 0.0], [0.06, 0.03, 0.03], true, GLASS);
    // Each wing, its root along the fuselage, its tip far back and out, the tip painted.
    for side in [-1.0f32, 1.0] {
        let edge = |t: f32| {
            let front = 0.25 + (-0.32 - 0.25) * t;
            let back = -0.25 + (-0.42 + 0.25) * t;
            let across = side * (0.04 + 0.46 * t);
            let (low, high) = (0.04, 0.06 - 0.012 * t);
            [
                [back, low, across],
                [front, low, across],
                [back, high, across],
                [front, high, across],
            ]
        };
        for ((t0, t1), paint) in [((0.0, 0.8), STEEL), ((0.8, 1.0), team)] {
            let ([b0, f0, bh0, fh0], [b1, f1, bh1, fh1]) = (edge(t0), edge(t1));
            model.hexa([b0, f0, f1, b1, bh0, fh0, fh1, bh1], paint);
        }
    }
    let fin = |a: f32, u: f32, c: f32| [a, u, c];
    model.hexa(
        [
            fin(-0.3, 0.1, -0.004),
            fin(-0.08, 0.1, -0.004),
            fin(-0.08, 0.1, 0.004),
            fin(-0.3, 0.1, 0.004),
            fin(-0.4, 0.3, -0.004),
            fin(-0.33, 0.3, -0.004),
            fin(-0.33, 0.3, 0.004),
            fin(-0.4, 0.3, 0.004),
        ],
        team,
    );
}

/// How an ornithopter's or a destroyer's wings are built: for each pair, how far along it is
/// and how far its tips reach; how far out their roots are and how high; their half chord at
/// root and tip, their thickness, how fast they beat and how far.
struct Wings {
    pairs: [(f32, f32); 2],
    root: f32,
    root_height: f32,
    chord: (f32, f32),
    thick: f32,
    beat_speed: f32,
    beat: (f32, f32),
    resting: f32,
}

/// Two pairs of wings, each a thin skin over three ribs, curving a little, with painted tips.
/// They beat in turn while it flies and lie still once it sits.
fn wings(model: &mut Model, unit: &Unit, time: f32, wings: &Wings, team: Paint) {
    const WING_BEATS: f32 = 30.0;
    for (&(along, reach), phase) in wings.pairs.iter().zip([0.0, PI]) {
        let tip_height = if unit.is_grounded() {
            wings.resting
        } else {
            wings.beat.0 + wings.beat.1 * (time * WING_BEATS * wings.beat_speed + phase).sin()
        };
        for side in [-1.0f32, 1.0] {
            // The front and back edges at a share of the way from root to tip, curving up a
            // little in the middle.
            let edge = |t: f32| {
                let chord = wings.chord.0 + (wings.chord.1 - wings.chord.0) * t;
                let height = wings.root_height
                    + (tip_height - wings.root_height) * t
                    + 0.04 * (t * PI).sin();
                let across = side * (wings.root + (reach - wings.root) * t);
                let thick = wings.thick * (1.0 - 0.5 * t);
                (
                    [along - chord, height, across],
                    [along + chord, height, across],
                    thick,
                )
            };
            let up = |p: [f32; 3], by: f32| [p[0], p[1] + by, p[2]];
            for ((t0, t1), paint) in [
                ((0.0, 0.45), STEEL),
                ((0.45, 0.82), STEEL),
                ((0.82, 1.0), team),
            ] {
                let ((b0, f0, k0), (b1, f1, k1)) = (edge(t0), edge(t1));
                model.hexa(
                    [
                        b0,
                        f0,
                        f1,
                        b1,
                        up(b0, k0),
                        up(f0, k0),
                        up(f1, k1),
                        up(b1, k1),
                    ],
                    paint,
                );
            }
            // The ribs on top, and a thicker leading edge.
            let (root_back, root_front, root_thick) = edge(0.0);
            let (tip_back, tip_front, tip_thick) = edge(1.0);
            let on_top = |a: [f32; 3], b: [f32; 3], t: f32, thick: f32| {
                up([0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t), thick)
            };
            if model.fine {
                for t in [0.15, 0.5, 0.85] {
                    model.bar(
                        on_top(root_back, root_front, t, root_thick),
                        on_top(tip_back, tip_front, t, tip_thick),
                        wings.thick * 0.5,
                        GUNMETAL,
                    );
                }
            }
            model.bar(
                up(root_front, root_thick * 0.5),
                up(tip_front, tip_thick * 0.5),
                wings.thick * 0.8,
                STEEL,
            );
        }
    }
}

/// A slim attack machine: a tapering fuselage with a painted stripe down its spine, a glass
/// canopy, a tail boom with a painted fin, rocket rails and two pairs of wings.
fn ornithopter(model: &mut Model, unit: &Unit, time: f32, team: Paint) {
    wings(
        model,
        unit,
        time,
        &Wings {
            pairs: [(0.16, 0.5), (-0.12, 0.42)],
            root: 0.06,
            root_height: 0.18,
            chord: (0.08, 0.05),
            thick: 0.012,
            beat_speed: 1.0,
            beat: (0.14, 0.22),
            resting: 0.1,
        },
        team,
    );
    model.loft(Loft {
        rings: &[
            [-0.52, 0.145, 0.0, 0.0],
            [-0.5, 0.145, 0.018, 0.018],
            [-0.22, 0.14, 0.03, 0.035],
            [-0.12, 0.13, 0.07, 0.08],
            [0.08, 0.12, 0.08, 0.09],
            [0.2, 0.115, 0.05, 0.055],
            [0.25, 0.11, 0.0, 0.0],
        ],
        sides: 12,
        paint: STEEL,
        top: Some(team),
    });
    model.ball([0.09, 0.17, 0.0], [0.08, 0.045, 0.05], true, GLASS);
    model.hexa(
        [
            [-0.52, 0.15, -0.004],
            [-0.42, 0.15, -0.004],
            [-0.42, 0.15, 0.004],
            [-0.52, 0.15, 0.004],
            [-0.54, 0.29, -0.004],
            [-0.49, 0.29, -0.004],
            [-0.49, 0.29, 0.004],
            [-0.54, 0.29, 0.004],
        ],
        team,
    );
    model.block([-0.48, 0.15, 0.0], [0.03, 0.004, 0.1], STEEL);
    for side in [-0.09, 0.09] {
        model.block([0.04, 0.06, side], [0.12, 0.01, 0.012], GUNMETAL);
    }
}

/// A heavy armored ornithopter: a broad fuselage with a painted band around its armored nose,
/// a slit canopy, a thick tail boom with twin painted fins, two big rocket pods and four heavy
/// wings beating slower.
fn destroyer(model: &mut Model, unit: &Unit, time: f32, team: Paint) {
    wings(
        model,
        unit,
        time,
        &Wings {
            pairs: [(0.26, 0.8), (-0.2, 0.68)],
            root: 0.11,
            root_height: 0.27,
            chord: (0.12, 0.07),
            thick: 0.018,
            beat_speed: 0.6,
            beat: (0.2, 0.24),
            resting: 0.14,
        },
        team,
    );
    model.loft(Loft {
        rings: &[
            [-0.64, 0.19, 0.0, 0.0],
            [-0.62, 0.19, 0.035, 0.035],
            [-0.34, 0.19, 0.06, 0.07],
            [-0.2, 0.18, 0.12, 0.15],
            [0.2, 0.17, 0.13, 0.17],
            [0.4, 0.17, 0.09, 0.11],
            [0.46, 0.16, 0.03, 0.04],
            [0.47, 0.16, 0.0, 0.0],
        ],
        sides: 14,
        paint: STEEL,
        top: None,
    });
    model.loft(Loft {
        rings: &[[0.29, 0.17, 0.125, 0.165], [0.37, 0.17, 0.105, 0.135]],
        sides: 14,
        paint: team,
        top: None,
    });
    model.block([0.24, 0.29, 0.0], [0.06, 0.012, 0.08], GLASS);
    for side in [-0.06, 0.06] {
        model.hexa(
            [
                [-0.64, 0.2, side - 0.005],
                [-0.5, 0.2, side - 0.005],
                [-0.5, 0.2, side + 0.005],
                [-0.64, 0.2, side + 0.005],
                [-0.67, 0.42, side - 0.005],
                [-0.6, 0.42, side - 0.005],
                [-0.6, 0.42, side + 0.005],
                [-0.67, 0.42, side + 0.005],
            ],
            team,
        );
    }
    for side in [-0.25, 0.25] {
        let height = 0.12;
        model.bevel(
            [0.06, 0.16, side * 0.7],
            [0.08, 0.02, 0.06],
            0.008,
            GUNMETAL,
        );
        model.tube(
            tube([-0.14, height, side], [0.3, height, side], 0.07).sides(12),
            GUNMETAL,
        );
        for (u, c) in [
            (-0.025, -0.035),
            (-0.025, 0.0),
            (-0.025, 0.035),
            (0.025, -0.035),
            (0.025, 0.0),
            (0.025, 0.035),
        ] {
            let at = [0.3, height + u, side + c];
            model.tube(
                tube(at, [0.303, at[1], at[2]], 0.016)
                    .sides(6)
                    .ends(false, true),
                BORE,
            );
        }
    }
}
