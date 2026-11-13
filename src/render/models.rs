//! The machines' models: bare metal with their owner's color on a few parts, built from bevelled
//! blocks, round tubes and bodies lofted from rings. Faces that neither the camera nor the sun
//! can see are left out as they are made.

use super::{Canvas, Point, cross, knee_of, offset, unit_vector};
use crate::battlefield::{CELL_SIDE, SUN};
use crate::game::Faction;
use crate::platform::{NO_PICTURE, WorldVertex};
use crate::units::{self, Role, Unit};
use crate::weapons::{
    RAGDOLL_CHEST, RAGDOLL_ELBOWS, RAGDOLL_FEET, RAGDOLL_HANDS, RAGDOLL_HEAD, RAGDOLL_KNEES,
    RAGDOLL_PELVIS, RAGDOLL_SECONDS, RAGDOLL_SINK_SECONDS, Ragdoll,
};
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
const CRANE_YELLOW: Paint = Paint {
    color: [0.93, 0.7, 0.08],
    shine: 0.35,
};
/// The dull olive of infantry's packs and launchers.
const INFANTRY_GEAR: Paint = Paint {
    color: [0.3, 0.32, 0.22],
    shine: 0.1,
};
const SKIN: Paint = Paint {
    color: [0.72, 0.56, 0.42],
    shine: 0.05,
};
const BOOTS: Paint = Paint {
    color: [0.1, 0.09, 0.08],
    shine: 0.1,
};
const HARD_HAT: Paint = Paint {
    color: [0.95, 0.85, 0.3],
    shine: 0.35,
};
const DETONATOR: Paint = Paint {
    color: [0.95, 0.08, 0.05],
    shine: 0.3,
};
const WOOD: Paint = Paint {
    color: [0.56, 0.38, 0.2],
    shine: 0.05,
};
const GOLD: Paint = Paint {
    color: [0.95, 0.75, 0.25],
    shine: 1.0,
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
        let [n0, n1, n2] = *normals;
        let normal = unit_vector(offset(offset(n0, n1, 1.0), n2, 1.0));
        if dot(normal, SUN) > SLACK {
            return true;
        }
        let [p0, p1, p2] = *points;
        let middle = offset(offset(p0, p1, 1.0), p2, 1.0).map(|c| c / 3.0);
        let faces = |eye: [f32; 3]| dot(normal, unit_vector(offset(eye, middle, -1.0))) > SLACK;
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
    /// Standing on the ground under the unit, turned to its heading, its model stretched to the
    /// original's size.
    pub fn on_ground(unit: &Unit) -> Self {
        Place::level(unit).stretched(unit.stretch())
    }

    /// Standing on the ground under the unit, turned to its heading.
    pub(super) fn level(unit: &Unit) -> Self {
        Place::turned([unit.x, unit.y, unit.z], unit.heading)
    }

    /// Standing at a point, level, turned to a heading.
    pub(super) fn turned(origin: [f32; 3], heading: f32) -> Self {
        let (sin, cos) = heading.sin_cos();
        Place {
            origin,
            along: [cos, 0.0, sin],
            up: [0.0, 1.0, 0.0],
            across: [-sin, 0.0, cos],
        }
    }

    /// Where the unit is up in the air, leaning with it into its turns, its model stretched to
    /// the original's size.
    pub fn in_air(unit: &Unit) -> Self {
        Place::flying(unit).stretched(unit.stretch())
    }

    /// Where the unit is up in the air, leaning with it into its turns.
    pub(super) fn flying(unit: &Unit) -> Self {
        let mut place = Place::level(unit);
        place.origin[1] += unit.altitude;
        let (sin, cos) = unit.bank.sin_cos();
        let (right, up) = (place.across, place.up);
        place.across = [0, 1, 2].map(|i| right[i] * cos - up[i] * sin);
        place.up = [0, 1, 2].map(|i| right[i] * sin + up[i] * cos);
        place
    }

    /// The same place, with what is made in it stretched this many times [along, up, across].
    fn stretched(self, [along, up, across]: [f32; 3]) -> Self {
        Place {
            along: self.along.map(|c| c * along),
            up: self.up.map(|c| c * up),
            across: self.across.map(|c| c * across),
            ..self
        }
    }

    /// A surface's normal given as [along, up, across], in the world: still at right angles to
    /// the surface however the place is stretched.
    fn normal(&self, [a, u, c]: [f32; 3]) -> [f32; 3] {
        let (along, up, across) = (self.along, self.up, self.across);
        let (to_along, to_up, to_across) =
            (cross(up, across), cross(across, along), cross(along, up));
        // A mirrored place turns the faces inside out, and these back with them.
        let turned = (0..3).map(|i| along[i] * to_along[i]).sum::<f32>().signum();
        unit_vector([0, 1, 2].map(|i| (to_along[i] * a + to_up[i] * u + to_across[i] * c) * turned))
    }

    /// A point given as [along, up, across] from the origin, in the world.
    pub fn point(&self, point: [f32; 3]) -> [f32; 3] {
        let [x, y, z] = self.way(point);
        [self.origin[0] + x, self.origin[1] + y, self.origin[2] + z]
    }

    /// A way given as [along, up, across], in the world. Spelled out, as it is made for every
    /// corner of every model.
    pub fn way(&self, [a, u, c]: [f32; 3]) -> [f32; 3] {
        let (along, up, across) = (self.along, self.up, self.across);
        [
            along[0] * a + up[0] * u + across[0] * c,
            along[1] * a + up[1] * u + across[1] * c,
            along[2] * a + up[2] * u + across[2] * c,
        ]
    }

    /// The same place with its origin moved by [along, up, across].
    pub(super) fn moved(self, by: [f32; 3]) -> Self {
        Place {
            origin: self.point(by),
            ..self
        }
    }

    /// The same place pitched nose up by an angle about an axis across it through a point given
    /// as [along, up, across], which stays where it was.
    fn pitched(self, about: [f32; 3], angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        let along = [0, 1, 2].map(|i| self.along[i] * cos + self.up[i] * sin);
        let up = [0, 1, 2].map(|i| self.up[i] * cos - self.along[i] * sin);
        let [a, u, _] = about;
        let pivot = self.point(about);
        Place {
            origin: [0, 1, 2].map(|i| pivot[i] - along[i] * a - up[i] * u),
            along,
            up,
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
    pub(super) fn at(&mut self, place: Place) -> Model<'_> {
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

    pub(super) fn triangle(&mut self, points: [[f32; 3]; 3], normals: [[f32; 3]; 3], paint: Paint) {
        self.pictured(points, normals, paint, [NO_PICTURE; 3], [0.0; 3]);
    }

    /// A triangle painted with a picture, where each corner is on it, with the house color for
    /// its house parts; `paint` for a backend that does not paint with pictures.
    pub(super) fn pictured(
        &mut self,
        points: [[f32; 3]; 3],
        normals: [[f32; 3]; 3],
        paint: Paint,
        picture: [[f32; 3]; 3],
        team: [f32; 3],
    ) {
        let points = points.map(|p| self.place.point(p));
        let normals = normals.map(|n| self.place.normal(n));
        if self.watchers.is_some_and(|w| !w.see(&points, &normals)) {
            return;
        }
        self.out.extend((0..3).map(|i| {
            let [x, y, z] = points[i];
            WorldVertex {
                picture: picture[i],
                team,
                ..WorldVertex::shiny((x, y, z), normals[i], paint.color, paint.shine)
            }
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
        let way = offset(tube.to, tube.from, -1.0);
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
            offset(u.map(|c| c * cos), v, sin)
        };
        let ring = |base: [f32; 3], radius: f32, o: [f32; 3]| offset(base, o, radius);
        let (near, far) = tube.radii;
        let slope = (near - far) / length;
        let normal = |o: [f32; 3]| unit_vector(offset(o, w, slope));
        // Each side starts where the one before it ended.
        let mut o1 = out(0.0);
        for side in 0..sides {
            let o0 = o1;
            o1 = out(side as f32 + 1.0);
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
        Role::Mongoose => &[[0.39, 0.58, 0.0]],
        Role::Minotaurus => &[[0.41, 1.15, -0.06], [0.41, 1.15, 0.06]],
        Role::AdvancedCarryall => &[[0.55, 0.03, -0.08], [0.55, 0.03, 0.08]],
        // One big lamp in the tip of its nose.
        Role::Carryall => &[[0.5, 0.07, 0.0]],
        Role::AirDefensePlatform => &[[0.41, 0.05, -0.27], [0.41, 0.05, 0.27]],
        Role::Ornithopter => &[[0.23, 0.08, 0.0]],
        Role::Destroyer => &[[0.45, 0.09, -0.06], [0.45, 0.09, 0.06]],
        // The searchlight on a tower's roof.
        Role::Tower | Role::GunTower => &[[0.14, 1.5, 0.0]],
        // A big headlamp on the front of each front track pod.
        Role::Mcv => &[[1.02, 0.34, -0.66], [1.02, 0.34, 0.66]],
        // Under the cockpit, at the end of the beam it sits on.
        Role::Harvester => &[[0.84, 0.6, 0.0]],
        Role::SandBike => &[[0.45, 0.2, 0.0]],
        Role::DiskSaw => &[[0.3, 0.27, -0.3], [0.3, 0.27, 0.3]],
        Role::FlameTank => &[[0.43, 0.2, 0.0]],
        Role::Devastator => &[[0.82, 1.9, -0.3], [0.82, 1.9, 0.3]],
        Role::Recon => &[[0.38, 0.42, 0.0]],
        Role::LaserTank => &[[0.48, 0.17, 0.0]],
        // On top of its right track.
        Role::Kobra => &[[0.36, 0.25, 0.33]],
        Role::RocketApc => &[[0.53, 0.2, 0.0]],
        Role::StealthApc => &[[0.56, 0.24, -0.19], [0.56, 0.24, 0.19]],
        _ => &[],
    }
}

/// How big a machine's lamp lenses are.
fn lamp_radius(role: Role) -> f32 {
    match role {
        Role::AdvancedCarryall => 0.045,
        Role::Carryall => 0.055,
        Role::Minotaurus => 0.035,
        Role::Devastator => 0.06,
        Role::Tower | Role::GunTower => 0.045,
        Role::Mcv => 0.075,
        Role::Harvester => 0.045,
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

/// Crates of money lying on the ground in a bunch: wooden boxes of a few sizes, each turned its
/// own way and bound with steel bands with a gold plate on its lid, one of them stacked on two
/// others.
pub fn draw_crate(canvas: &mut Canvas, (x, y, z): Point) {
    // Each box as where its middle is from the bunch's middle (across, up from its foot,
    // ahead), how far it reaches each way, and how far it is turned.
    const BOXES: [([f32; 3], f32, f32); 5] = [
        ([-0.2, 0.0, -0.1], 0.2, 0.78),
        ([0.2, 0.0, -0.08], 0.17, 0.95),
        ([-0.02, 0.0, 0.26], 0.15, 0.5),
        ([0.0, 0.4, -0.09], 0.15, 1.1),
        ([0.32, 0.0, 0.28], 0.11, 0.2),
    ];
    for ([across, up, ahead], half, turn) in BOXES {
        let (sin, cos) = f32::sin_cos(turn);
        let place = Place {
            origin: [x + across * 45.25, y + up * 45.25, z + ahead * 45.25],
            along: [cos, 0.0, sin],
            up: [0.0, 1.0, 0.0],
            across: [-sin, 0.0, cos],
        }
        .stretched([45.25; 3]);
        let model = &mut Model::new(canvas, place);
        model.bevel([0.0, half, 0.0], [half; 3], 0.02, WOOD);
        for along in [-0.55, 0.55] {
            let band = [0.015, half + 0.004, half + 0.004];
            model.block([along * half, half, 0.0], band, STEEL);
        }
        let plate = half * 0.32;
        model.block([0.0, half * 2.0, 0.0], [plate, 0.006, plate], GOLD);
    }
}

/// A pop-up turret: a low round collar in the ground with a bulge in its middle, which rises
/// with the turret's gun when it has something to fire at.
pub fn draw_popup_turret(canvas: &mut Canvas, unit: &Unit, color: [f32; 3], walls: [f32; 3]) {
    let (trim, collar) = (team(color), team(walls));
    let model = &mut Model::new(canvas, Place::on_ground(unit));
    let round = model.sides(20);
    let rise = unit.popped * units::TURRET_RISE;
    model.tube(
        tube([0.0, 0.0, 0.0], [0.0, 0.12, 0.0], 0.5)
            .taper(0.44)
            .sides(round),
        collar,
    );
    model.tube(
        tube([0.0, 0.12, 0.0], [0.0, 0.13, 0.0], 0.43).sides(round),
        trim,
    );
    if rise > 0.0 {
        model.tube(
            tube([0.0, 0.1, 0.0], [0.0, 0.14 + rise, 0.0], 0.3).sides(round),
            STEEL,
        );
    }
    let middle = [0.0, 0.14 + rise, 0.0];
    model.ball(middle, [0.32, 0.17, 0.32], true, STEEL);
    let (reach, half, paint) = if unit.role == Role::GasTower {
        (0.28, 0.1, self::team([0.35, 0.9, 0.25]))
    } else {
        (0.45, 0.035, GLARE)
    };
    if unit.popped > 0.05 {
        let up = middle[1] + 0.07;
        let tip = [0.22 + reach * unit.popped, up, 0.0];
        model.tube(tube([0.18, up, 0.0], tip, half).sides(8), GUNMETAL);
        model.tube(
            tube(tip, [tip[0] + 0.012, up, 0.0], half * 1.15)
                .sides(8)
                .ends(false, true),
            paint,
        );
    }
}

/// The beams a machine casts now: none from one sitting on the ground that only flies.
pub fn beams(unit: &Unit) -> Vec<Beam> {
    let spots = lamp_spots(unit.role);
    let flies_only = matches!(unit.role, Role::Ornithopter | Role::Destroyer);
    // A deployed MCV puts out its headlamps.
    if spots.is_empty() || flies_only && unit.is_grounded() || unit.deployed {
        return Vec::new();
    }
    let place = if unit.kind == units::UnitKind::Aircraft {
        Place::in_air(unit)
    } else {
        Place::on_ground(unit)
    };
    let (reach, spread) = match unit.role {
        Role::AdvancedCarryall | Role::Carryall => (362.0, 117.7),
        Role::AirDefensePlatform => (203.6, 36.2),
        Role::Ornithopter | Role::Destroyer => (226.3, 54.31),
        Role::Minotaurus => (158.4, 45.25),
        Role::Mcv => (203.6, 67.88),
        // A searchlight throws a long, narrow beam far over the ground as the tower looks out.
        Role::Tower | Role::GunTower => (362.0, 40.73),
        _ if spots.len() == 1 => (117.7, 45.25),
        _ => (117.7, 33.94),
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
                half: lamp_radius(unit.role) * unit.stretch()[2],
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
        Role::Harvester => harvester(model, unit, team),
        Role::Catapult => catapult(model, unit, team),
        Role::RocketLauncher => rocket_launcher(model, unit, team),
        Role::Mongoose => strider(model, unit, team),
        Role::Minotaurus => heavy_strider(model, unit, team),
        Role::AdvancedCarryall => advanced_carryall(model, team),
        Role::Carryall => carryall(model, team),
        Role::AirDefensePlatform => air_defense_platform(model, team),
        Role::AirDrone => air_drone(model, team),
        Role::Ornithopter => ornithopter(model, unit, time, team),
        Role::Destroyer => destroyer(model, unit, time, team),
        Role::SandBike => sand_bike(model, team),
        Role::DiskSaw => disk_saw(model, unit, time, team),
        Role::FlameTank => flame_tank(model, unit, team),
        Role::Devastator => huge_walker(model, unit, team),
        Role::Recon => recon(model, unit, team),
        Role::LaserTank => laser_tank(model, time, team),
        Role::Kobra => siege_tank(model, unit, team),
        Role::RocketApc => rocket_apc(model, time, team),
        Role::AirMine => air_mine(model, time, team),
        Role::EyeInTheSky => eye_in_the_sky(model, time, team),
        Role::StealthApc => stealth_apc(model, team),
        _ => return,
    }
    let radius = lamp_radius(unit.role);
    for &spot in lamp_spots(unit.role) {
        model.lamp(spot, radius);
    }
}

/// A square lattice between two points, thinning from one half width to another: four chords,
/// a ring at each bay and, with the detail for it, a diagonal across each face.
fn lattice(
    model: &mut Model,
    from: [f32; 3],
    to: [f32; 3],
    (near, far): (f32, f32),
    bays: usize,
    paint: Paint,
) {
    const ROD: f32 = 0.6336;
    let way = [0, 1, 2].map(|i| to[i] - from[i]);
    let length = unit_vector(way);
    let other = if length[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let u = unit_vector(cross(length, other));
    let v = cross(length, u);
    let corner = |bay: usize, which: usize| {
        let t = bay as f32 / bays as f32;
        let half = near + (far - near) * t;
        let (su, sv) = [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)][which % 4];
        [0, 1, 2].map(|i| from[i] + way[i] * t + (u[i] * su + v[i] * sv) * half)
    };
    for which in 0..4 {
        model.bar(corner(0, which), corner(bays, which), ROD, paint);
        for bay in 0..=bays {
            model.bar(corner(bay, which), corner(bay, which + 1), ROD * 0.7, paint);
        }
        if model.fine {
            for bay in 0..bays {
                let (a, b) = if bay % 2 == 0 {
                    (corner(bay, which), corner(bay + 1, which + 1))
                } else {
                    (corner(bay, which + 1), corner(bay + 1, which))
                };
                model.bar(a, b, ROD * 0.7, paint);
            }
        }
    }
}

/// One side of the scaffolding, this high: posts, a ledger every level and cross braces.
fn scaffold(model: &mut Model, from: [f32; 2], to: [f32; 2], height: f32) {
    const LEVEL: f32 = 22.63;
    const ROD: f32 = 0.5883;
    if height <= 0.9051 {
        return;
    }
    let bays = (((to[0] - from[0]).hypot(to[1] - from[1]) / 33.94).ceil() as usize).max(1);
    let at = |bay: usize, y: f32| {
        let t = bay as f32 / bays as f32;
        [
            from[0] + (to[0] - from[0]) * t,
            y,
            from[1] + (to[1] - from[1]) * t,
        ]
    };
    for bay in 0..=bays {
        model.bar(at(bay, 0.0), at(bay, height), ROD, STEEL);
    }
    let levels = (height / LEVEL).ceil() as usize;
    let level = |n: usize| (n as f32 * LEVEL).min(height);
    for n in 1..=levels {
        model.bar(at(0, level(n)), at(bays, level(n)), ROD, STEEL);
        if model.fine {
            for bay in 0..bays {
                let (low, high) = (level(n - 1), level(n));
                let (a, b) = if (bay + n) % 2 == 0 {
                    (at(bay, low), at(bay + 1, high))
                } else {
                    (at(bay + 1, low), at(bay, high))
                };
                model.bar(a, b, ROD * 0.7, STEEL);
            }
        }
    }
}

/// A tower crane on its truss mast, its jib turned this way, as much put up as `appear` says.
fn crane(
    model: &mut Model,
    [a, c]: [f32; 2],
    height: f32,
    jib: f32,
    slew: f32,
    done: f32,
    appear: f32,
) {
    let mast = height * appear;
    model.bevel([a, 2.263, c], [14.48, 2.263, 14.48], 0.9051, GUNMETAL);
    lattice(
        model,
        [a, 3.62, c],
        [a, mast, c],
        (5.883, 5.883),
        7,
        CRANE_YELLOW,
    );
    let (sin, cos) = slew.sin_cos();
    let jib = jib * appear;
    let at = |reach: f32, up: f32| [a + cos * reach, mast + up, c + sin * reach];
    model.bevel([a, mast + 3.62, c], [7.693, 4.073, 7.693], 0.9051, GUNMETAL);
    // The arrow, thinning to its tip, and the short arm behind that balances it.
    lattice(
        model,
        at(5.431, 7.241),
        at(jib, 7.241),
        (4.978, 1.131),
        8,
        CRANE_YELLOW,
    );
    lattice(
        model,
        at(-5.431, 7.241),
        at(-jib * 0.36, 7.241),
        (3.62, 2.715),
        3,
        CRANE_YELLOW,
    );
    model.tube(
        tube(at(-jib * 0.36, 3.62), at(-jib * 0.2, 3.62), 5.883)
            .sides(4)
            .faceted(),
        GUNMETAL,
    );
    // The top of the tower and the ties from it that hold both arms up.
    let peak = at(0.0, 38.47 * appear);
    model.bar(at(0.0, 7.241), peak, 0.9956, CRANE_YELLOW);
    model.bar(peak, at(jib * 0.72, 9.051), 0.3168, STEEL);
    model.bar(peak, at(-jib * 0.33, 9.051), 0.3168, STEEL);
    // The trolley running along the arrow, and what it hoists.
    let reach = jib * (0.45 + 0.35 * (done * TAU).sin());
    let hook_up = mast * (0.42 + 0.3 * (done * TAU * 1.5).cos());
    let trolley = at(reach, 3.62);
    model.bevel(trolley, [3.62, 1.584, 3.62], 0.4525, GUNMETAL);
    let hook = [trolley[0], hook_up, trolley[2]];
    model.bar(trolley, hook, 0.2715, BORE);
    model.bevel(hook, [2.263, 2.263, 2.263], 0.4525, CRANE_YELLOW);
    let girder = |reach: f32| [hook[0] + cos * reach, hook[1] - 3.62, hook[2] + sin * reach];
    model.bar(girder(-18.1), girder(18.1), 1.584, STEEL);
}

/// What puts a building up in its one second, in three stages: scaffolding on all four sides
/// and two cranes go up, the cranes turn opposite ways while the building grows, and then it
/// all comes down and leaves the building bare.
pub fn draw_construction(canvas: &mut Canvas, unit: &Unit) {
    let smooth = |t: f32| {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let done = unit.construction_done();
    let (up, down) = (units::CONSTRUCTION_UP, units::CONSTRUCTION_DOWN);
    let work = ((done - up) / (down - up)).clamp(0.0, 1.0);
    let (sin, cos) = std::f32::consts::FRAC_PI_4.sin_cos();
    // Along the grid, as its squares are.
    let place = Place {
        origin: [unit.x, unit.y, unit.z],
        along: [cos, 0.0, sin],
        up: [0.0, 1.0, 0.0],
        across: [-sin, 0.0, cos],
    };
    let model = &mut Model::new(canvas, place);
    let (cols, rows) = unit.plot().map_or(unit.role.plot_size(unit.house), |plot| {
        (plot.cols, plot.rows)
    });
    let half_a = cols as f32 * CELL_SIDE / 2.0 + 6.788;
    let half_c = rows as f32 * CELL_SIDE / 2.0 + 6.788;
    let top = unit.role.building_top(unit.house) + 11.31;
    let corners = [
        [-half_a, -half_c],
        [half_a, -half_c],
        [half_a, half_c],
        [-half_a, half_c],
    ];
    for side in 0..4 {
        // Each side a little after the one before, so that they do not move as one.
        let late = side as f32 * 0.03;
        let rising = smooth((done - late) / (up - 0.09).max(0.01));
        let falling = smooth((1.0 - done - late) / (1.0 - down - 0.09).max(0.01));
        let height = top * rising.min(falling);
        scaffold(model, corners[side], corners[(side + 1) % 4], height);
    }
    let appear = smooth(done / up).min(smooth((1.0 - done) / (1.0 - down)));
    if appear <= 0.0 {
        return;
    }
    let (height, jib, out) = (top + 67.88, half_a.max(half_c) * 1.35 + 36.2, 24.89);
    crane(
        model,
        [-half_a - out, -half_c - out],
        height,
        jib,
        0.4 + work * TAU * 0.75,
        work,
        appear,
    );
    crane(
        model,
        [half_a + out, half_c + out],
        height - 20.36,
        jib * 0.87,
        3.6 - work * TAU * 0.75,
        (work + 0.3) % 1.0,
        appear,
    );
}

/// Where a soldier's hands are, as [along, up, across], with across positive to its right.
struct Grip {
    right: [f32; 3],
    left: [f32; 3],
}

/// A hand hanging at the side, swinging as the soldier walks.
fn hanging_hand(across: f32, swing: f32) -> [f32; 3] {
    [0.03 + swing, 0.27, across]
}

fn rifle_stock(model: &mut Model, up: f32) {
    model.bar([-0.02, up - 0.008, 0.02], [0.08, up, 0.02], 0.016, GUNMETAL);
}

/// Draws what a soldier carries, from a place that bobs with it, and says where its hands go.
fn soldier_gear(model: &mut Model, unit: &Unit, team: Paint, swing: f32) -> Grip {
    match (unit.role, unit.deployed) {
        (Role::Sniper, _) => {
            model.tube(
                tube([0.05, 0.42, 0.02], [0.56, 0.42, 0.02], 0.009).sides(6),
                GUNMETAL,
            );
            model.bar([0.02, 0.41, 0.02], [0.26, 0.42, 0.02], 0.02, GUNMETAL);
            model.tube(
                tube([0.1, 0.46, 0.02], [0.25, 0.46, 0.02], 0.016).sides(6),
                BORE,
            );
            rifle_stock(model, 0.41);
            for side in [-0.03, 0.03] {
                model.bar(
                    [0.42, 0.42, 0.02],
                    [0.44, 0.3, 0.02 + side],
                    0.004,
                    GUNMETAL,
                );
            }
            Grip {
                right: [0.1, 0.395, 0.04],
                left: [0.27, 0.4, 0.015],
            }
        }
        (Role::RegularInfantry, _) => {
            model.bevel([0.17, 0.4, 0.02], [0.11, 0.028, 0.018], 0.006, GUNMETAL);
            model.tube(
                tube([0.28, 0.405, 0.02], [0.33, 0.405, 0.02], 0.014).sides(6),
                BORE,
            );
            model.ball([0.18, 0.35, 0.02], [0.05, 0.05, 0.03], false, INFANTRY_GEAR);
            rifle_stock(model, 0.4);
            Grip {
                right: [0.09, 0.385, 0.04],
                left: [0.25, 0.385, 0.01],
            }
        }
        (Role::KInfantry, true) => {
            model.bevel([0.05, 0.1, 0.0], [0.3, 0.08, 0.2], 0.02, INFANTRY_GEAR);
            model.tube(
                tube([0.05, 0.2, 0.0], [0.75, 0.18, 0.0], 0.04).sides(8),
                GUNMETAL,
            );
            model.tube(
                tube([0.68, 0.183, 0.0], [0.75, 0.18, 0.0], 0.055).sides(8),
                STEEL,
            );
            model.bevel([0.12, 0.25, 0.0], [0.1, 0.06, 0.1], 0.01, GUNMETAL);
            Grip {
                right: [0.14, 0.27, 0.09],
                left: [0.14, 0.27, -0.09],
            }
        }
        (Role::MortarInfantry, true) => {
            model.bevel([0.3, 0.02, 0.0], [0.14, 0.02, 0.14], 0.005, INFANTRY_GEAR);
            model.tube(
                tube([0.26, 0.04, 0.0], [0.38, 0.52, 0.0], 0.04).sides(8),
                GUNMETAL,
            );
            model.bar([0.32, 0.02, 0.1], [0.34, 0.26, 0.015], 0.008, GUNMETAL);
            model.bar([0.32, 0.02, -0.1], [0.34, 0.26, -0.015], 0.008, GUNMETAL);
            Grip {
                right: [0.3, 0.3, 0.045],
                left: [0.33, 0.38, -0.045],
            }
        }
        (Role::KInfantry | Role::MortarInfantry, false) => {
            if unit.role == Role::MortarInfantry {
                model.tube(
                    tube([-0.13, 0.22, 0.0], [-0.18, 0.66, 0.0], 0.04).sides(8),
                    GUNMETAL,
                );
            }
            model.bevel([0.2, 0.425, 0.02], [0.05, 0.016, 0.01], 0.003, GUNMETAL);
            model.bar([0.16, 0.42, 0.02], [0.15, 0.38, 0.02], 0.009, GUNMETAL);
            Grip {
                right: [0.15, 0.39, 0.03],
                left: hanging_hand(-0.13, -swing),
            }
        }
        (Role::RocketInfantry, _) => {
            let tip = [0.34, 0.56, 0.1];
            model.tube(
                tube([-0.26, 0.55, 0.1], tip, 0.05)
                    .sides(8)
                    .ends(false, false),
                INFANTRY_GEAR,
            );
            model.tube(tube([0.335, 0.56, 0.1], tip, 0.042).sides(8), BORE);
            model.bevel([0.12, 0.49, 0.1], [0.03, 0.03, 0.012], 0.005, GUNMETAL);
            Grip {
                right: [0.05, 0.48, 0.1],
                left: [0.2, 0.5, 0.09],
            }
        }
        (Role::FlameInfantry | Role::GasInfantry, _) => {
            let [r, g, b] = if unit.role == Role::GasInfantry {
                super::GAS_COLOR
            } else {
                super::FIRE_COLOR
            };
            let tanks = Paint {
                color: [r, g, b],
                shine: 0.4,
            };
            for side in [-0.05, 0.05] {
                model.tube(
                    tube([-0.14, 0.24, side], [-0.14, 0.52, side], 0.045).sides(8),
                    tanks,
                );
            }
            model.bar([-0.12, 0.26, 0.04], [0.1, 0.37, 0.04], 0.008, RUBBER);
            model.tube(
                tube([0.06, 0.385, 0.03], [0.32, 0.385, 0.02], 0.016).sides(6),
                GUNMETAL,
            );
            model.tube(
                tube([0.31, 0.385, 0.02], [0.36, 0.385, 0.02], 0.028).sides(6),
                tanks,
            );
            Grip {
                right: [0.1, 0.375, 0.04],
                left: [0.24, 0.375, 0.01],
            }
        }
        (Role::SuicideInfantry, _) => {
            for n in 0..6 {
                let (sin, cos) = (n as f32 / 6.0 * TAU).sin_cos();
                let at = [cos * 0.09, 0.33, sin * 0.09];
                model.tube(
                    tube([at[0], 0.26, at[2]], [at[0], 0.4, at[2]], 0.026).sides(6),
                    INFANTRY_GEAR,
                );
            }
            let raised = [0.13, 0.62, 0.11];
            model.bevel(raised, [0.018, 0.035, 0.018], 0.004, DETONATOR);
            Grip {
                right: raised,
                left: hanging_hand(-0.13, -swing),
            }
        }
        (Role::Scout, _) => {
            for side in [-0.03, 0.03] {
                model.tube(
                    tube([0.09, 0.56, side], [0.17, 0.56, side], 0.02).sides(6),
                    BORE,
                );
            }
            model.bevel([-0.1, 0.4, 0.0], [0.04, 0.08, 0.06], 0.01, INFANTRY_GEAR);
            model.bar([-0.1, 0.48, 0.03], [-0.13, 0.82, 0.03], 0.003, GUNMETAL);
            Grip {
                right: [0.12, 0.54, 0.05],
                left: [0.12, 0.54, -0.05],
            }
        }
        (Role::Engineer, _) => {
            let carried = hanging_hand(-0.14, -swing * 0.4);
            model.bevel(
                [carried[0], carried[1] - 0.07, carried[2]],
                [0.075, 0.05, 0.035],
                0.01,
                team,
            );
            model.bar([0.15, 0.37, 0.11], [0.3, 0.42, 0.11], 0.01, STEEL);
            model.bevel([0.31, 0.425, 0.11], [0.025, 0.03, 0.012], 0.004, STEEL);
            Grip {
                right: [0.15, 0.37, 0.11],
                left: carried,
            }
        }
        _ => {
            model.bevel([0.16, 0.4, 0.02], [0.095, 0.018, 0.012], 0.004, GUNMETAL);
            model.tube(
                tube([0.25, 0.402, 0.02], [0.33, 0.402, 0.02], 0.008).sides(6),
                GUNMETAL,
            );
            model.bar([0.16, 0.38, 0.02], [0.18, 0.33, 0.02], 0.012, BORE);
            rifle_stock(model, 0.4);
            Grip {
                right: [0.09, 0.385, 0.04],
                left: [0.23, 0.385, 0.01],
            }
        }
    }
}

/// What a soldier wears on its head, around a head at this height.
fn headgear(model: &mut Model, role: Role, up: f32, team: Paint) {
    match role {
        Role::Engineer => {
            model.ball([0.0, up + 0.02, 0.0], [0.082, 0.055, 0.082], true, HARD_HAT);
        }
        Role::RegularInfantry | Role::KInfantry | Role::RocketInfantry => {
            model.ball(
                [0.0, up + 0.015, 0.0],
                [0.066, 0.056, 0.066],
                true,
                GUNMETAL,
            );
        }
        Role::FlameInfantry | Role::GasInfantry => {
            model.ball([0.0, up, 0.0], [0.066, 0.066, 0.066], false, INFANTRY_GEAR);
            model.block([0.055, up + 0.005, 0.0], [0.018, 0.02, 0.04], GLASS);
        }
        Role::Sniper | Role::Scout => {
            model.ball([0.0, up + 0.012, 0.0], [0.06, 0.04, 0.06], true, team);
            model.block([0.06, up + 0.02, 0.0], [0.03, 0.005, 0.04], team);
        }
        _ => {
            model.ball([0.0, up + 0.018, 0.0], [0.058, 0.046, 0.058], true, team);
        }
    }
}

/// A soldier on two jointed legs, its arms reaching for what it carries: it steps and swings as
/// it walks, breathes as it waits and kneels when it has dug in.
pub fn draw_infantry(canvas: &mut Canvas, unit: &Unit, time: f32, color: [f32; 3]) {
    const PACE: f32 = 9.0;
    const THIGH: f32 = 0.16;
    const SHIN: f32 = 0.16;
    const UPPER_ARM: f32 = 0.15;
    const FOREARM: f32 = 0.15;
    let team = team(color);
    // Coming down by parachute, it hangs in the air under it.
    let mut place = Place::on_ground(unit);
    place.origin[1] += unit.altitude;
    let model = &mut Model::new(canvas, place);
    if unit.is_parachuting() {
        parachute(model, team);
    }
    let kneeling = unit.deployed && matches!(unit.role, Role::KInfantry | Role::MortarInfantry);
    let phase = unit.stride * PACE;
    let walking = unit.is_moving() && !kneeling;
    let (bob, swing, breath) = if walking {
        (phase.sin().abs() * 0.014, phase.sin() * 0.09, 0.0)
    } else {
        let breath = (time * 2.2 + unit.id as f32 * 1.3).sin() * 0.006;
        (0.0, 0.0, breath)
    };
    let small = if unit.role == Role::Scout { 0.9 } else { 1.0 };
    let hip_up = if kneeling { 0.17 } else { 0.3 * small + bob };
    let lean = if walking { 0.03 } else { 0.0 };
    let chest = [lean, hip_up + 0.2 * small + breath, 0.0];
    for (side, step, ahead) in [(-0.055, 0.0, 0.14), (0.055, PI, -0.2)] {
        let (foot_a, lift) = if kneeling {
            (ahead, 0.0)
        } else if walking {
            let turn = phase + step;
            (turn.sin() * 0.12, turn.cos().max(0.0) * 0.07)
        } else {
            (0.0, 0.0)
        };
        let (knee_a, knee_u) = super::knee_of((0.0, hip_up), (foot_a, lift + 0.03), THIGH, SHIN);
        let hip = [0.0, hip_up, side];
        let knee = [knee_a, knee_u, side];
        let ankle = [foot_a, lift + 0.03, side];
        model.tube(tube(hip, knee, 0.037).taper(0.03).sides(6), team);
        model.tube(tube(knee, ankle, 0.029).taper(0.023).sides(6), BOOTS);
        model.bevel(
            [foot_a + 0.025, lift + 0.018, side],
            [0.052, 0.018, 0.028],
            0.006,
            BOOTS,
        );
    }
    let waist = [0.0, hip_up, 0.0];
    model.tube(tube(waist, chest, 0.07).taper(0.085).sides(8), team);
    let head_up = chest[1] + 0.1 * small;
    model.tube(
        tube(chest, [lean + 0.004, head_up - 0.03, 0.0], 0.026).sides(6),
        SKIN,
    );
    model.ball(
        [lean + 0.01, head_up, 0.0],
        [0.05, 0.056, 0.05],
        false,
        SKIN,
    );
    let rise = chest[1] - 0.5;
    let moved = place.moved([0.0, rise, 0.0]);
    let grip = soldier_gear(&mut model.at(moved), unit, team, swing);
    headgear(
        &mut model.at(place.moved([lean + 0.01, 0.0, 0.0])),
        unit.role,
        head_up,
        team,
    );
    for (hand, side) in [(grip.right, 0.1), (grip.left, -0.1)] {
        let hand = [hand[0], hand[1] + rise, hand[2]];
        let shoulder = [lean, chest[1] - 0.005, side];
        let (elbow_a, elbow_u) = super::knee_of(
            (hand[0], hand[1]),
            (shoulder[0], shoulder[1]),
            FOREARM,
            UPPER_ARM,
        );
        let elbow = [
            elbow_a,
            elbow_u,
            (hand[2] + shoulder[2]) / 2.0 + side * 0.25,
        ];
        model.ball(shoulder, [0.036; 3], false, team);
        model.tube(tube(shoulder, elbow, 0.026).taper(0.022).sides(6), team);
        model.tube(tube(elbow, hand, 0.021).taper(0.018).sides(6), team);
        model.ball(hand, [0.024; 3], false, SKIN);
    }
}

/// A painted canopy high over a soldier's head, on four lines down to the shoulders.
fn parachute(model: &mut Model, team: Paint) {
    const TOP: f32 = 1.15;
    const HALF: f32 = 0.42;
    model.ball([0.0, TOP, 0.0], [HALF, 0.18, HALF], true, team);
    for (along, across) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        let rim = [along * HALF * 0.7, TOP, across * HALF * 0.7];
        model.bar([along * 0.03, 0.52, across * 0.08], rim, 0.004, WHITE);
    }
}

/// A fallen soldier as its joints lie now, sinking away at the end.
pub fn draw_ragdoll(canvas: &mut Canvas, ragdoll: &Ragdoll, color: [f32; 3]) {
    let team = team(color);
    let sink = (ragdoll.age - (RAGDOLL_SECONDS - RAGDOLL_SINK_SECONDS)).max(0.0) * 18.1;
    let place = Place {
        origin: [0.0, -sink, 0.0],
        along: [1.0, 0.0, 0.0],
        up: [0.0, 1.0, 0.0],
        across: [0.0, 0.0, 1.0],
    };
    let model = &mut Model::new(canvas, place);
    let joint = |i: usize| {
        let (x, y, z) = ragdoll.joints[i];
        [x, y, z]
    };
    let chest = joint(RAGDOLL_CHEST);
    let pelvis = joint(RAGDOLL_PELVIS);
    let head = joint(RAGDOLL_HEAD);
    // As thick as the soldier was drawn.
    let r = |radius: f32| radius * ragdoll.unit.stretch()[1];
    model.tube(tube(pelvis, chest, r(0.07)).taper(r(0.085)).sides(8), team);
    model.tube(tube(chest, head, r(0.026)).sides(6), SKIN);
    model.ball(head, [r(0.05), r(0.056), r(0.05)], false, SKIN);
    for side in 0..2 {
        let (elbow, hand) = (joint(RAGDOLL_ELBOWS[side]), joint(RAGDOLL_HANDS[side]));
        model.tube(tube(chest, elbow, r(0.026)).taper(r(0.022)).sides(6), team);
        model.tube(tube(elbow, hand, r(0.021)).taper(r(0.018)).sides(6), team);
        model.ball(hand, [r(0.024); 3], false, SKIN);
        let (knee, foot) = (joint(RAGDOLL_KNEES[side]), joint(RAGDOLL_FEET[side]));
        model.tube(tube(pelvis, knee, r(0.037)).taper(r(0.03)).sides(6), team);
        model.tube(tube(knee, foot, r(0.029)).taper(r(0.023)).sides(6), BOOTS);
        model.ball(foot, [r(0.04), r(0.025), r(0.04)], false, BOOTS);
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

/// A harvester: four linked tracks in pods under a domed shell like a turtle's, its plates in
/// the owner's color, a scoop low in front, and a beam reaching forward and up out of the shell
/// like a turtle's neck, with the driver's cockpit on its end and a headlamp under it.
fn harvester(model: &mut Model, unit: &Unit, team: Paint) {
    const APART: f32 = 0.42;
    for (rear, front) in [(-0.62, -0.14), (0.1, 0.56)] {
        let track = Track {
            rear,
            front,
            height: 0.12,
            radius: 0.11,
            half_width: 0.09,
            road_wheels: 3,
            links: 14,
        };
        tracks(model, unit, &track, APART);
    }
    model.bevel([-0.03, 0.25, 0.0], [0.62, 0.06, 0.34], 0.02, GUNMETAL);
    // The shell: a low dome over a lip, with a plate on its crown and a ring of them around it.
    let (middle, half) = ([-0.08, 0.3, 0.0], [0.6, 0.38, 0.5]);
    model.ball([middle[0], 0.31, 0.0], [0.64, 0.05, 0.54], false, GUNMETAL);
    model.ball(middle, half, true, STEEL);
    model.ball(
        [middle[0], middle[1] + half[1] - 0.04, 0.0],
        [0.2, 0.07, 0.17],
        true,
        team,
    );
    if model.fine {
        for k in 0..6 {
            let (sin, cos) = (k as f32 / 6.0 * TAU).sin_cos();
            let (along, across) = (0.36 * cos, 0.3 * sin);
            let out = (along / half[0]).powi(2) + (across / half[2]).powi(2);
            let up = middle[1] + half[1] * (1.0 - out).max(0.0).sqrt() - 0.04;
            model.ball(
                [middle[0] + along, up, across],
                [0.13, 0.06, 0.11],
                true,
                team,
            );
        }
    }
    // The scoop it pushes through the sand, with its teeth.
    model.bevel([0.66, 0.1, 0.0], [0.06, 0.07, 0.36], 0.015, GUNMETAL);
    if model.fine {
        for across in [-0.27, -0.09, 0.09, 0.27] {
            model.block([0.74, 0.05, across], [0.03, 0.02, 0.03], STEEL);
        }
    }
    // The neck: a beam out of the shell's front, rising as it goes.
    model.hexa(
        [
            [0.3, 0.32, -0.11],
            [0.72, 0.5, -0.09],
            [0.72, 0.5, 0.09],
            [0.3, 0.32, 0.11],
            [0.3, 0.5, -0.11],
            [0.72, 0.64, -0.09],
            [0.72, 0.64, 0.09],
            [0.3, 0.5, 0.11],
        ],
        GUNMETAL,
    );
    // The cockpit on its end, painted, with its windscreen looking ahead.
    model.bevel([0.7, 0.74, 0.0], [0.14, 0.1, 0.15], 0.03, team);
    model.bevel([0.82, 0.77, 0.0], [0.03, 0.055, 0.12], 0.01, GLASS);
    model.tube(
        tube([0.66, 0.84, 0.0], [0.66, 0.88, 0.0], 0.022).sides(8),
        BEACON,
    );
    // The chute at the back it empties out of.
    model.bevel([-0.68, 0.32, 0.0], [0.05, 0.08, 0.16], 0.015, BRONZE);
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
    /// How high the hips stand, as a share of the length of the leg: low, for thighs that lie
    /// nearly flat out of them.
    stand: f32,
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
    let hip_height = (legs.thigh + legs.shin) * legs.stand;
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

/// A small walking missile carrier: a long rectangular body up on two bird legs, with a lamp
/// in front and a separate launcher of four missile tubes on its back.
fn strider(model: &mut Model, unit: &Unit, team: Paint) {
    legs(
        model,
        unit,
        &Legs {
            stand: 0.92,
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
    model.bevel([0.0, 0.62, 0.0], [0.38, 0.1, 0.14], 0.02, STEEL);
    model.block([-0.05, 0.722, 0.0], [0.26, 0.004, 0.09], team);
    model.block([0.382, 0.66, 0.0], [0.004, 0.035, 0.09], GLASS);
    // The launcher stands apart on a post on its back.
    model.tube(
        tube([-0.14, 0.72, 0.0], [-0.14, 0.78, 0.0], 0.05).sides(8),
        GUNMETAL,
    );
    let launcher_up = 0.86;
    model.bevel(
        [-0.14, launcher_up, 0.0],
        [0.17, 0.09, 0.13],
        0.012,
        GUNMETAL,
    );
    model.block([-0.14, launcher_up, 0.132], [0.13, 0.04, 0.003], team);
    model.block([-0.14, launcher_up, -0.132], [0.13, 0.04, 0.003], team);
    for (u, c) in [(-0.04, -0.06), (-0.04, 0.06), (0.04, -0.06), (0.04, 0.06)] {
        let at = [0.03, launcher_up + u, c];
        model.tube(
            tube(at, [0.034, at[1], at[2]], 0.03)
                .sides(6)
                .ends(false, true),
            BORE,
        );
    }
}

/// A walking gun platform: a broad body on two big legs with armored knees, a raised painted
/// top, a sensor head with two lamps, four cannons that slide back as they fire, and exhausts.
fn heavy_strider(model: &mut Model, unit: &Unit, team: Paint) {
    /// How far a cannon runs back into its sleeve when it fires.
    const RECOIL_TRAVEL: f32 = 0.55;
    /// Longer all told than the body it is mounted on.
    const BARREL_HALF: f32 = 0.525;
    legs(
        model,
        unit,
        &Legs {
            stand: 0.54,
            side: 0.34,
            thigh: 0.62,
            shin: 1.05,
            swing: 0.3,
            pace: 2.6,
            width: 0.19,
            knee_caps: Some(team),
            pistons: 2,
        },
    );
    // The platform tilts to aim, and its cannons elevate further on their hinge.
    let [pivot_along, pivot_up] = units::PLATFORM_PIVOT;
    let platform = model.place.pitched(
        [pivot_along, pivot_up, 0.0],
        unit.tube_pitch * units::PLATFORM_SHARE_OF_PITCH,
    );
    let height = units::CANNON_HEIGHT + 0.03;
    let cannons = platform.pitched(
        [units::CANNON_HINGE, units::CANNON_HEIGHT, 0.0],
        unit.tube_pitch * (1.0 - units::PLATFORM_SHARE_OF_PITCH),
    );
    let mut model = model.at(platform);
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
        [units::CANNON_HINGE, height, 0.0],
        [0.035, 0.07, 0.38],
        GUNMETAL,
    );
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
    let mut model = model.at(cannons);
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
            tube(
                [units::CANNON_HINGE, height, side],
                [0.82, height, side],
                0.05,
            ),
            STEEL,
        );
    }
}

/// A long lifting frame: a lattice spine with a painted band, an engine nacelle on a pylon at
/// each side, a clamp frame underneath and two floodlights under its nose.
fn advanced_carryall(model: &mut Model, team: Paint) {
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

/// A carryall: a slim fuselage with a glass cockpit and a big lamp under its nose. Ahead, low
/// under the nose, is a wide wing with a flap over it between two painted end plates, like a
/// racing car's front wing; in the middle a straight wing with a jet at each end, its intake
/// ringed with paint; and at the tail a wing between twin fins with painted tips.
fn carryall(model: &mut Model, team: Paint) {
    model.loft(Loft {
        rings: &[
            [-0.58, 0.13, 0.0, 0.0],
            [-0.54, 0.13, 0.03, 0.035],
            [-0.3, 0.12, 0.055, 0.065],
            [0.08, 0.11, 0.075, 0.085],
            [0.3, 0.1, 0.06, 0.07],
            [0.44, 0.08, 0.035, 0.04],
            [0.5, 0.07, 0.0, 0.0],
        ],
        sides: 12,
        paint: STEEL,
        top: None,
    });
    model.ball([0.24, 0.16, 0.0], [0.13, 0.05, 0.05], true, GLASS);
    // A thin flat slab across, `half` thick, from its back edge to its front edge, each given
    // as [along, up], and from one side to the other.
    let slab = |model: &mut Model, back: [f32; 2], front: [f32; 2], half: f32, (c0, c1), paint| {
        let at = |[a, u]: [f32; 2], up: f32, c: f32| [a, u + up, c];
        model.hexa(
            [
                at(back, -half, c0),
                at(front, -half, c0),
                at(front, -half, c1),
                at(back, -half, c1),
                at(back, half, c0),
                at(front, half, c0),
                at(front, half, c1),
                at(back, half, c1),
            ],
            paint,
        );
    };
    // The front wing: its main plane, the flap above and behind it, and the end plates.
    let span = (-0.3, 0.3);
    slab(model, [0.4, 0.035], [0.54, 0.03], 0.007, span, STEEL);
    slab(model, [0.36, 0.08], [0.44, 0.055], 0.006, span, STEEL);
    for c in [-0.06, 0.06] {
        model.bar([0.46, 0.035, c], [0.42, 0.085, c * 0.6], 0.008, GUNMETAL);
    }
    for c in [-0.3, 0.3] {
        model.hexa(
            [
                [0.34, 0.02, c - 0.006],
                [0.55, 0.02, c - 0.006],
                [0.55, 0.02, c + 0.006],
                [0.34, 0.02, c + 0.006],
                [0.34, 0.11, c - 0.006],
                [0.5, 0.07, c - 0.006],
                [0.5, 0.07, c + 0.006],
                [0.34, 0.11, c + 0.006],
            ],
            team,
        );
    }
    // The middle wing and its two jets.
    let [nozzle, up, out] = units::CARRYALL_JET_NOZZLE;
    for side in [-1.0f32, 1.0] {
        slab(
            model,
            [-0.06, up],
            [0.13, up],
            0.01,
            (side * 0.06, side * out),
            STEEL,
        );
        let c = side * out;
        let intake = nozzle + 0.32;
        model.tube(
            tube([nozzle, up, c], [intake, up, c], 0.05).sides(12),
            STEEL,
        );
        model.tube(
            tube([intake, up, c], [intake + 0.03, up, c], 0.054)
                .sides(12)
                .ends(false, false),
            team,
        );
        model.tube(
            tube([intake - 0.02, up, c], [intake + 0.025, up, c], 0.036)
                .sides(10)
                .ends(false, true),
            BORE,
        );
        model.tube(
            tube([nozzle, up, c], [nozzle - 0.04, up, c], 0.045)
                .taper(0.036)
                .sides(10)
                .ends(false, false),
            GUNMETAL,
        );
        model.tube(
            tube([nozzle + 0.01, up, c], [nozzle - 0.035, up, c], 0.03)
                .sides(8)
                .ends(false, true),
            BORE,
        );
    }
    // The tail wing between its twin fins.
    let tail = (-0.2, 0.2);
    slab(model, [-0.58, 0.14], [-0.44, 0.14], 0.007, tail, STEEL);
    for c in [-0.2, 0.2] {
        let fin = |a: f32, u: f32, side: f32| [a, u, c + side * 0.006];
        for ((low, high), paint) in [((0.13, 0.27), STEEL), ((0.27, 0.32), team)] {
            let back = |u: f32| -0.6 - (u - 0.13) * 0.4;
            let front = |u: f32| -0.43 - (u - 0.13) * 0.6;
            model.hexa(
                [
                    fin(back(low), low, -1.0),
                    fin(front(low), low, -1.0),
                    fin(front(low), low, 1.0),
                    fin(back(low), low, 1.0),
                    fin(back(high), high, -1.0),
                    fin(front(high), high, -1.0),
                    fin(front(high), high, 1.0),
                    fin(back(high), high, 1.0),
                ],
                paint,
            );
        }
    }
    // The clamps the cables hang from.
    for along in [-0.1, 0.1] {
        for c in [-0.1, 0.1] {
            model.block([along, 0.05, c], [0.025, 0.025, 0.018], GUNMETAL);
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
    for spot in AIR_DEFENSE_GLARES {
        model.bevel(spot, [0.016, 0.016, 0.016], 0.004, GLARE);
    }
}

/// Where the red lights on an air defense platform are, as [along, up, across].
const AIR_DEFENSE_GLARES: [[f32; 3]; 2] = [[-0.34, 0.125, -0.27], [-0.34, 0.125, 0.27]];

/// Where the red lights on an air defense platform are in the world.
pub fn air_defense_glares(unit: &Unit) -> [Point; 2] {
    let place = Place::in_air(unit);
    AIR_DEFENSE_GLARES.map(|spot| {
        let [x, y, z] = place.point(spot);
        (x, y, z)
    })
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

/// How high the roof of each house's MCV is, where its searchlight stands.
fn mcv_roof(faction: Faction) -> f32 {
    match faction {
        Faction::Atreides => 1.2,
        Faction::Harkonnen => 1.07,
        Faction::Ordos => 0.9,
    }
}

/// Where an MCV's searchlight swivels, as [along, up, across].
fn mcv_searchlight_spot(faction: Faction) -> [f32; 3] {
    [-0.35, mcv_roof(faction) + 0.12, 0.0]
}

/// Where the top of a deployed MCV's pipe is, as [along, up, across].
fn mcv_pipe_spot(faction: Faction) -> [f32; 3] {
    [-0.72, mcv_roof(faction) + 1.05, 0.3]
}

/// Which way an MCV's searchlight points now, in the world: slowly round and round.
fn mcv_sweep(unit: &Unit, time: f32) -> f32 {
    time * MCV_SWEEP_SPEED + unit.id as f32 * 1.7
}

/// Radians per second an MCV's searchlight sweeps around.
const MCV_SWEEP_SPEED: f32 = 0.45;

/// Where the top of a deployed MCV's pipe is in the world, where its smoke leaves.
pub fn mcv_pipe_top(unit: &Unit, faction: Faction) -> Point {
    let [x, y, z] = Place::on_ground(unit).point(mcv_pipe_spot(faction));
    (x, y, z)
}

/// The red glares at the top of a deployed MCV's pipe, in the world, either side of it.
pub fn mcv_glares(unit: &Unit, faction: Faction) -> [Point; 2] {
    let [a, u, c] = mcv_pipe_spot(faction);
    let place = Place::on_ground(unit);
    [-1.0, 1.0].map(|side| {
        let [x, y, z] = place.point([a, u - 0.12, c + side * 0.13]);
        (x, y, z)
    })
}

/// The beam of an MCV's searchlight: a long one, slowly sweeping the ground all around it.
pub fn mcv_searchlight(unit: &Unit, faction: Faction, time: f32) -> Beam {
    let [x, y, z] = Place::on_ground(unit).point(mcv_searchlight_spot(faction));
    let (sin, cos) = mcv_sweep(unit, time).sin_cos();
    Beam {
        lens: (x + cos * 5.431, y, z + sin * 5.431),
        ahead: [cos, 0.0, sin],
        right: [-sin, 0.0, cos],
        half: 2.715,
        reach: 407.3,
        spread: 49.78,
    }
}

/// A huge drill on a hinge at an MCV's nose: lying level while it drives, then tipped down and
/// bored into the ground as it deploys, its two spiral flights showing it turn.
fn mcv_drill(model: &mut Model, unit: &Unit, team: Paint) {
    const LENGTH: f32 = 1.7;
    const RADIUS: f32 = 0.3;
    const PLUNGE: f32 = 0.45;
    const FLIGHTS: usize = 2;
    const STEPS: usize = 18;
    let [along, up] = units::MCV_DRILL_PIVOT;
    let pose = unit.drill_pose();
    model.bevel([along - 0.06, up, 0.0], [0.18, 0.2, 0.3], 0.03, GUNMETAL);
    let base = model.place;
    let (sin, cos) = pose.tilt.sin_cos();
    let way = base.way([cos, -sin, 0.0]);
    let pivot = base.point([along, up, 0.0]);
    let place = Place {
        origin: [0, 1, 2].map(|i| pivot[i] + way[i] * pose.plunge * PLUNGE),
        along: way,
        up: base.way([sin, cos, 0.0]),
        across: base.across,
    };
    let mut drill = model.at(place);
    drill.tube(
        tube([-0.3, 0.0, 0.0], [0.05, 0.0, 0.0], 0.13).sides(12),
        STEEL,
    );
    drill.tube(
        tube([0.05, 0.0, 0.0], [0.16, 0.0, 0.0], RADIUS + 0.04).sides(14),
        team,
    );
    drill.tube(
        tube([0.16, 0.0, 0.0], [LENGTH, 0.0, 0.0], RADIUS)
            .taper(0.015)
            .sides(14),
        GUNMETAL,
    );
    let flight_point = |flight: usize, step: usize| {
        let share = step as f32 / STEPS as f32;
        let angle = pose.spin + share * TAU * 3.0 + flight as f32 * TAU / FLIGHTS as f32;
        let radius = (RADIUS + 0.035) * (1.0 - share) + 0.015;
        let (sin, cos) = angle.sin_cos();
        [0.16 + share * (LENGTH - 0.16), radius * sin, radius * cos]
    };
    for flight in 0..FLIGHTS {
        for step in 0..STEPS {
            let (from, to) = (flight_point(flight, step), flight_point(flight, step + 1));
            drill.bar(from, to, 0.028, STEEL);
        }
    }
}

const GLARE: Paint = Paint {
    color: [1.0, 0.12, 0.08],
    shine: 0.0,
};

/// The huge main vehicle: four linked tracks in pods, a big headlamp on the front of each front
/// pod, a body in its house's style and a searchlight on the roof. Deployed, it stands on four
/// legs with a smoking pipe at the back.
pub fn draw_mcv(canvas: &mut Canvas, unit: &Unit, faction: Faction, time: f32, color: [f32; 3]) {
    let team = team(color);
    let model = &mut Model::new(canvas, Place::on_ground(unit));
    for (rear, front) in [(0.3, 0.95), (-0.95, -0.3)] {
        let track = Track {
            rear,
            front,
            height: 0.17,
            radius: 0.15,
            half_width: 0.13,
            road_wheels: 3,
            links: 18,
        };
        tracks(model, unit, &track, MCV_TRACKS_APART);
        // A fender over each pod.
        for side in [-1.0, 1.0] {
            let middle = [(rear + front) / 2.0, 0.37, side * MCV_TRACKS_APART];
            model.bevel(middle, [0.42, 0.035, 0.16], 0.015, STEEL);
        }
    }
    // The chassis that ties the four pods together.
    model.bevel([0.0, 0.36, 0.0], [1.05, 0.12, 0.5], 0.03, GUNMETAL);
    let roof = mcv_roof(faction);
    match faction {
        Faction::Atreides => {
            // Smooth and rounded: a lofted body with the owner's color along its top, and a
            // domed cab with a wide windscreen.
            let rings = [
                [-1.08, 0.72, 0.16, 0.46],
                [-0.85, 0.74, 0.26, 0.56],
                [0.35, 0.74, 0.28, 0.58],
                [0.85, 0.7, 0.22, 0.5],
                [1.08, 0.62, 0.1, 0.34],
            ];
            model.loft(Loft {
                rings: &rings,
                sides: 14,
                paint: STEEL,
                top: Some(team),
            });
            model.ball([0.45, 0.98, 0.0], [0.34, 0.22, 0.34], true, STEEL);
            model.bevel([0.74, 0.98, 0.0], [0.03, 0.07, 0.24], 0.01, GLASS);
            model.tube(tube([-0.35, 0.98, 0.0], [-0.35, roof, 0.0], 0.11), GUNMETAL);
        }
        Faction::Harkonnen => {
            // Brutal and angular: a heavy armored wedge with a ram in front, the owner's color
            // on its side plates, and spikes along its top edges.
            let corner = |a: f32, u: f32, c: f32| [a, u, c];
            model.hexa(
                [
                    corner(-1.08, 0.46, -0.6),
                    corner(1.0, 0.46, -0.6),
                    corner(1.0, 0.46, 0.6),
                    corner(-1.08, 0.46, 0.6),
                    corner(-0.98, roof, -0.44),
                    corner(0.5, roof, -0.44),
                    corner(0.5, roof, 0.44),
                    corner(-0.98, roof, 0.44),
                ],
                GUNMETAL,
            );
            model.hexa(
                [
                    corner(1.0, 0.18, -0.5),
                    corner(1.22, 0.18, -0.5),
                    corner(1.22, 0.18, 0.5),
                    corner(1.0, 0.18, 0.5),
                    corner(1.0, 0.5, -0.56),
                    corner(1.08, 0.5, -0.56),
                    corner(1.08, 0.5, 0.56),
                    corner(1.0, 0.5, 0.56),
                ],
                STEEL,
            );
            for side in [-1.0, 1.0] {
                model.block([-0.25, 0.72, side * 0.585], [0.55, 0.13, 0.012], team);
                if model.fine {
                    for along in [-0.8, -0.4, 0.0, 0.4] {
                        let foot = [along, roof, side * 0.42];
                        let tip = [along - 0.06, roof + 0.16, side * 0.5];
                        model.tube(tube(foot, tip, 0.035).taper(0.0).sides(4), BRONZE);
                    }
                }
            }
            model.bevel([0.55, roof - 0.12, 0.0], [0.03, 0.06, 0.3], 0.0, GLASS);
        }
        Faction::Ordos => {
            // Low and sleek: a tapering wedge with swept fins in the owner's color and a raised
            // module at the back.
            let corner = |a: f32, u: f32, c: f32| [a, u, c];
            model.hexa(
                [
                    corner(-1.06, 0.46, -0.58),
                    corner(1.14, 0.46, -0.58),
                    corner(1.14, 0.46, 0.58),
                    corner(-1.06, 0.46, 0.58),
                    corner(-0.92, 0.8, -0.36),
                    corner(0.32, 0.8, -0.36),
                    corner(0.32, 0.8, 0.36),
                    corner(-0.92, 0.8, 0.36),
                ],
                STEEL,
            );
            model.bevel([-0.5, 0.85, 0.0], [0.38, 0.06, 0.3], 0.03, GUNMETAL);
            for side in [-1.0, 1.0] {
                let corners = [
                    [-0.95, 0.62, side * 0.5],
                    [-0.2, 0.62, side * 0.5],
                    [-0.2, 0.64, side * 0.53],
                    [-0.95, 0.64, side * 0.53],
                    [-0.98, 0.98, side * 0.66],
                    [-0.72, 0.98, side * 0.66],
                    [-0.72, 1.0, side * 0.68],
                    [-0.98, 1.0, side * 0.68],
                ];
                model.hexa(corners, team);
            }
            model.bevel([0.34, 0.7, 0.0], [0.03, 0.06, 0.26], 0.0, GLASS);
        }
    }
    // The searchlight on its swivel, pointing the way it sweeps.
    let spot = mcv_searchlight_spot(faction);
    model.tube(
        tube([spot[0], roof, 0.0], [spot[0], spot[1], 0.0], 0.05).sides(8),
        GUNMETAL,
    );
    let (sin, cos) = (mcv_sweep(unit, time) - unit.heading).sin_cos();
    let base = model.place;
    let swivel = Place {
        origin: base.point(spot),
        along: base.way([cos, 0.0, sin]),
        up: base.up,
        across: base.way([-sin, 0.0, cos]),
    };
    model.at(swivel).lamp([0.1, 0.0, 0.0], 0.06);
    for &lamp in lamp_spots(Role::Mcv) {
        model.lamp(lamp, lamp_radius(Role::Mcv));
    }
    mcv_drill(model, unit, team);
    if !unit.deployed {
        return;
    }
    // Four legs braced out to the ground at the corners.
    for (along, side) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
        let hip = [along * 0.98, 0.42, side * 0.5];
        let foot = [along * 1.32, 0.04, side * 0.98];
        model.tube(tube(hip, foot, 0.05).sides(8), STEEL);
        model.bevel(foot, [0.12, 0.04, 0.12], 0.015, GUNMETAL);
    }
    // The pipe, smoking from its top, with a red glare either side of it.
    let top = mcv_pipe_spot(faction);
    let foot = [top[0], 0.85, top[2]];
    model.tube(tube(foot, top, 0.085).sides(10).ends(true, false), GUNMETAL);
    model.tube(
        tube([top[0], top[1] - 0.08, top[2]], top, 0.105).sides(10),
        BRONZE,
    );
    model.tube(
        tube(top, [top[0], top[1] + 0.002, top[2]], 0.07).sides(10),
        BORE,
    );
    for side in [-1.0, 1.0] {
        let at = [top[0], top[1] - 0.12, top[2] + side * 0.13];
        model.ball(at, [0.03, 0.03, 0.03], false, GLARE);
    }
}

/// How far an MCV's tracks are from its middle line.
const MCV_TRACKS_APART: f32 = 0.66;

/// A fast sand bike: a flat teardrop, round at the nose and drawn out to a point behind, on a
/// wheel at each end, with a rifle on its nose.
fn sand_bike(model: &mut Model, team: Paint) {
    let rings = [
        [-0.42, 0.2, 0.0, 0.0],
        [-0.34, 0.2, 0.065, 0.15],
        [-0.2, 0.2, 0.085, 0.2],
        [0.05, 0.2, 0.08, 0.18],
        [0.3, 0.2, 0.05, 0.1],
        [0.47, 0.2, 0.0, 0.0],
    ];
    model.loft(Loft {
        rings: &rings,
        sides: 14,
        paint: STEEL,
        top: Some(team),
    });
    for along in [0.26, -0.24] {
        model.wheel([along, 0.11, 0.0], 0.11, 0.06, 12, RUBBER);
    }
    model.ball([0.06, 0.27, 0.0], [0.12, 0.06, 0.09], true, GLASS);
    model.tube(
        tube([0.18, 0.26, 0.0], [0.5, 0.2, 0.0], 0.016).sides(6),
        GUNMETAL,
    );
}

/// A disk saw: a huge blade spinning upright in the middle, with nothing behind it but a small
/// pod on each side, each on its own track with a rifle and a lamp.
fn disk_saw(model: &mut Model, unit: &Unit, time: f32, team: Paint) {
    const POD_SIDE: f32 = 0.3;
    const RADIUS: f32 = 0.46;
    let track = Track {
        rear: -0.3,
        front: 0.3,
        height: 0.1,
        radius: 0.085,
        half_width: 0.06,
        road_wheels: 4,
        links: 18,
    };
    tracks(model, unit, &track, POD_SIDE);
    let hub = [0.0, 0.54, 0.0];
    for side in [-POD_SIDE, POD_SIDE] {
        model.bevel([0.0, 0.25, side], [0.24, 0.08, 0.07], 0.02, GUNMETAL);
        model.block([-0.02, 0.335, side], [0.16, 0.006, 0.05], team);
        model.bar([0.0, 0.3, side], [0.0, hub[1], side], 0.03, STEEL);
        model.bevel([0.14, 0.36, side], [0.08, 0.03, 0.03], 0.01, GUNMETAL);
        model.tube(
            tube([0.2, 0.36, side], [0.5, 0.36, side], 0.016).sides(6),
            GUNMETAL,
        );
    }
    // The axle between the pods, and the blade on it with teeth around its rim as it spins.
    model.tube(
        tube([0.0, hub[1], -POD_SIDE], [0.0, hub[1], POD_SIDE], 0.035).sides(8),
        GUNMETAL,
    );
    let blade =
        tube([hub[0], hub[1], -0.014], [hub[0], hub[1], 0.014], RADIUS).sides(model.sides(28));
    model.tube(blade, STEEL);
    model.tube(
        tube([hub[0], hub[1], -0.05], [hub[0], hub[1], 0.05], 0.08).sides(8),
        BRONZE,
    );
    let teeth = if model.fine { 20 } else { 10 };
    let spin = time * 14.0;
    for n in 0..teeth {
        let angle = spin + n as f32 / teeth as f32 * TAU;
        let (sin, cos) = angle.sin_cos();
        let (ahead_sin, ahead_cos) = (angle + 0.2).sin_cos();
        let at = |r: f32, s: f32, c: f32| [hub[0] + c * r, hub[1] + s * r, 0.0];
        let (a, b, c) = (
            at(RADIUS - 0.01, sin, cos),
            at(RADIUS - 0.01, ahead_sin, ahead_cos),
            at(RADIUS + 0.07, sin, cos),
        );
        model.triangle([a, b, c], [[0.0, 0.0, 1.0]; 3], STEEL);
        model.triangle([a, c, b], [[0.0, 0.0, -1.0]; 3], STEEL);
    }
}

/// A flame tank: a tracked hull with two fuel tanks on its back and a flamethrower at each
/// front corner, one aimed at ten and one at two o'clock.
fn flame_tank(model: &mut Model, unit: &Unit, team: Paint) {
    let track = Track {
        rear: -0.34,
        front: 0.34,
        height: 0.1,
        radius: 0.085,
        half_width: 0.06,
        road_wheels: 5,
        links: 20,
    };
    tracks(model, unit, &track, 0.205);
    model.bevel([0.0, 0.22, 0.0], [0.42, 0.09, 0.2], 0.03, STEEL);
    for side in [-1.0, 1.0] {
        model.block([0.0, 0.25, side * 0.282], [0.36, 0.04, 0.006], team);
        model.tube(
            tube([-0.4, 0.42, side * 0.1], [0.05, 0.42, side * 0.1], 0.085).sides(10),
            BRONZE,
        );
    }
    for way in crate::weapons::FLAME_TANK_NOZZLES {
        let (sin, cos) = way.sin_cos();
        let from = [0.25 + cos * 0.08, 0.3, sin * 0.12];
        let to = [0.25 + cos * 0.3, 0.3, sin * 0.34];
        model.tube(tube(from, to, 0.035).taper(0.045).sides(8), GUNMETAL);
        model.tube(
            tube(to, [to[0] + cos * 0.004, to[1], to[2] + sin * 0.004], 0.04).sides(8),
            BORE,
        );
    }
}

/// A huge tank: long tracks with six road wheels, a broad hull and a big turret with twin
/// cannons.
fn huge_walker(model: &mut Model, unit: &Unit, team: Paint) {
    const HIP_UP: f32 = 2.0;
    let plasma = self::team([0.35, 0.55, 1.0]);
    let moving = unit.is_moving();
    // One leg reaching forward and two back, each stepping in its turn.
    for (i, way) in [0.0_f32, 2.2, -2.2].into_iter().enumerate() {
        let (sin, cos) = way.sin_cos();
        let (swing, lift) = if moving {
            let phase = unit.stride * 3.0 + i as f32 * TAU / 3.0;
            (0.4 * phase.sin(), 0.35 * phase.cos().max(0.0))
        } else {
            (0.0, 0.0)
        };
        let hip = [cos * 0.5, HIP_UP, sin * 0.5];
        let knee = [cos * 1.25, 2.55, sin * 1.25];
        let foot = [cos * 1.5 + swing, lift + 0.05, sin * 1.5];
        model.ball(hip, [0.24; 3], false, BRONZE);
        model.tube(tube(hip, knee, 0.15).taper(0.12).sides(8), STEEL);
        model.ball(knee, [0.19; 3], false, BRONZE);
        model.tube(tube(knee, foot, 0.13).taper(0.09).sides(8), GUNMETAL);
        let sole = [foot[0], lift, foot[2]];
        model.tube(tube(sole, foot, 0.32).taper(0.2).sides(10), GUNMETAL);
    }
    model.bevel([0.0, HIP_UP, 0.0], [0.75, 0.38, 0.62], 0.06, GUNMETAL);
    model.bevel([-0.05, HIP_UP + 0.5, 0.0], [0.55, 0.14, 0.5], 0.04, STEEL);
    model.block([0.1, HIP_UP + 0.645, 0.3], [0.2, 0.006, 0.09], team);
    model.block([0.76, HIP_UP + 0.05, 0.0], [0.012, 0.11, 0.26], GLASS);
    // A plasma gun on each shoulder, ringed with glowing coils.
    let (tip, up, side) = units::WALKER_PLASMA_TIP;
    for side in [-side, side] {
        let shoulder = [0.1, up, side];
        model.ball(shoulder, [0.22; 3], false, BRONZE);
        model.tube(
            tube(shoulder, [tip, up, side], 0.1).taper(0.07).sides(10),
            GUNMETAL,
        );
        for n in 0..3 {
            let along = 0.6 + n as f32 * 0.25;
            model.tube(
                tube([along, up, side], [along + 0.04, up, side], 0.14).sides(10),
                plasma,
            );
        }
    }
    // A rack of three rockets on its back.
    let (ahead, rack) = (units::WALKER_ROCKET_AHEAD, units::WALKER_ROCKET_UP);
    model.bevel([ahead - 0.33, rack, 0.0], [0.33, 0.16, 0.4], 0.03, GUNMETAL);
    for side in units::WALKER_ROCKET_SIDES {
        let at = [ahead, rack, side];
        model.tube(
            tube(at, [ahead + 0.01, rack, side], 0.09)
                .sides(8)
                .ends(false, true),
            BORE,
        );
    }
}

/// A recon machine on four long insect legs, two reaching forward and two back, that step as
/// it runs and as it turns.
fn recon(model: &mut Model, unit: &Unit, team: Paint) {
    const THIGH: f32 = 0.32;
    const SHIN: f32 = 0.42;
    let body_height = 0.42;
    model.ball([0.0, body_height, 0.0], [0.26, 0.11, 0.17], false, STEEL);
    model.ball(
        [0.0, body_height + 0.02, 0.0],
        [0.22, 0.09, 0.14],
        true,
        team,
    );
    model.ball(
        [0.27, body_height, 0.0],
        [0.11, 0.08, 0.09],
        false,
        GUNMETAL,
    );
    model.ball(
        [0.33, body_height + 0.02, 0.0],
        [0.04, 0.035, 0.07],
        false,
        GLASS,
    );
    model.tube(
        tube(
            [0.3, body_height - 0.07, 0.0],
            [0.55, body_height - 0.07, 0.0],
            0.014,
        )
        .sides(6),
        GUNMETAL,
    );
    let moving = unit.is_moving();
    for (forward, side, step) in [
        (1.0, -1.0, 0.0),
        (1.0, 1.0, PI),
        (-1.0, -1.0, PI),
        (-1.0, 1.0, 0.0),
    ] {
        let phase = unit.stride * 4.0 + step;
        let (swing, lift) = if moving || unit.stride > 0.0 {
            (0.1 * phase.sin(), 0.07 * phase.cos().max(0.0))
        } else {
            (0.0, 0.0)
        };
        let hip = [forward * 0.14, body_height, side * 0.12];
        let foot = [forward * 0.6 + swing, lift, side * 0.38];
        // The knee rises high above the line from hip to foot, like an insect's.
        let reach = (foot[0] - hip[0]).hypot(foot[2] - hip[2]);
        let knee2 = knee_of((0.0, hip[1]), (reach, foot[1]), THIGH, SHIN);
        let t = knee2.0 / reach.max(0.001);
        let knee = [
            hip[0] + (foot[0] - hip[0]) * t,
            knee2.1,
            hip[2] + (foot[2] - hip[2]) * t,
        ];
        model.ball(hip, [0.035; 3], false, BRONZE);
        model.tube(tube(hip, knee, 0.025).sides(6), STEEL);
        model.ball(knee, [0.03; 3], false, BRONZE);
        model.tube(tube(knee, foot, 0.02).taper(0.01).sides(6), GUNMETAL);
    }
}

/// A laser tank gliding just above the ground: a flat hover skirt under a smooth wedge with
/// the owner's color on top, and a laser on a dome in the middle.
fn laser_tank(model: &mut Model, time: f32, team: Paint) {
    let float = 0.02 * (time * 3.0).sin();
    model.bevel([0.0, 0.1 + float, 0.0], [0.44, 0.05, 0.3], 0.04, GUNMETAL);
    let rings = [
        [-0.46, 0.2 + float, 0.07, 0.26],
        [-0.2, 0.22 + float, 0.11, 0.3],
        [0.25, 0.22 + float, 0.1, 0.28],
        [0.48, 0.18 + float, 0.03, 0.14],
    ];
    model.loft(Loft {
        rings: &rings,
        sides: 12,
        paint: STEEL,
        top: Some(team),
    });
    model.ball(
        [-0.02, 0.33 + float, 0.0],
        [0.13, 0.12, 0.13],
        true,
        GUNMETAL,
    );
    model.tube(
        tube([0.0, 0.42 + float, 0.0], [0.0, 0.52 + float, 0.0], 0.03).sides(8),
        STEEL,
    );
    model.ball([0.0, 0.54 + float, 0.0], [0.045; 3], false, LASER_CRYSTAL);
}

/// An APC's armored box: a flat back and sides, its roof painted, and its front sloping
/// down from the roof to the nose. Returns how high its roof is.
fn apc_hull(model: &mut Model, foot: f32, team: Paint) -> f32 {
    let top = foot + 0.21;
    let (tail, nose, brow) = (-0.5, 0.55, 0.28);
    let (low, high) = (0.3, 0.25);
    model.hexa(
        [
            [tail, foot, -low],
            [nose, foot, -low],
            [nose, foot, low],
            [tail, foot, low],
            [tail + 0.03, top, -high],
            [brow, top, -high],
            [brow, top, high],
            [tail + 0.03, top, high],
        ],
        STEEL,
    );
    model.block([-0.1, top + 0.004, 0.0], [0.3, 0.004, 0.18], team);
    // The rear door the soldiers get in and out by.
    model.block(
        [tail - 0.004, foot + 0.1, 0.0],
        [0.004, 0.08, 0.13],
        GUNMETAL,
    );
    let slope = unit_vector([top - foot, nose - brow, 0.0]);
    let screen = |t: f32, c: f32| [nose - (nose - brow) * t + 0.004, foot + (top - foot) * t, c];
    model.quad(
        [
            screen(0.55, -0.16),
            screen(0.55, 0.16),
            screen(0.9, 0.16),
            screen(0.9, -0.16),
        ],
        slope,
        GLASS,
    );
    top
}

/// House O's rocket APC: an armored box hovering on a skirt, with a rocket pod on each side
/// of its roof.
fn rocket_apc(model: &mut Model, time: f32, team: Paint) {
    let float = 0.02 * (time * 3.0).sin();
    model.bevel([0.0, 0.11 + float, 0.0], [0.5, 0.05, 0.31], 0.04, GUNMETAL);
    let top = apc_hull(model, 0.16 + float, team);
    let [ahead, up, side] = units::APC_ROCKET_TIP;
    let up = up + float;
    for side in [-side, side] {
        model.block([0.05, top + 0.02, side * 0.8], [0.08, 0.02, 0.04], GUNMETAL);
        model.tube(
            tube([-0.18, up, side], [ahead - 0.02, up, side], 0.07).sides(model.sides(10)),
            team,
        );
        model.tube(
            tube([ahead - 0.02, up, side], [ahead, up, side], 0.05).sides(model.sides(8)),
            BORE,
        );
    }
}

/// House A's stealth APC: an armored box on six wheels, with twin guns in a small turret.
fn stealth_apc(model: &mut Model, team: Paint) {
    for along in [-0.32, 0.0, 0.32] {
        for side in [-1.0, 1.0] {
            model.wheel([along, 0.1, side * 0.25], 0.1, 0.08, 12, RUBBER);
            model.wheel([along, 0.1, side * 0.25], 0.05, 0.088, 8, STEEL);
        }
    }
    model.block([0.0, 0.15, 0.0], [0.48, 0.04, 0.2], GUNMETAL);
    let top = apc_hull(model, 0.19, team);
    let [ahead, up, side] = units::APC_GUN_TIP;
    model.bevel([0.05, up, 0.0], [0.13, up - top, 0.1], 0.02, team);
    for side in [-side, side] {
        model.tube(
            tube([0.15, up, side], [ahead, up, side], 0.014).sides(6),
            GUNMETAL,
        );
    }
}

/// An air mine: a round pod bobbing in the air with a painted band, and six rockets around
/// it pointing out, slowly turning, their noses painted.
fn air_mine(model: &mut Model, time: f32, team: Paint) {
    const ROCKETS: usize = 6;
    let bob = 0.03 * (time * 2.0).sin();
    model.ball([0.0, bob, 0.0], [0.15; 3], false, GUNMETAL);
    let band = tube([0.0, bob - 0.02, 0.0], [0.0, bob + 0.02, 0.0], 0.16);
    model.tube(band.sides(model.sides(14)), team);
    let spin = time * 0.6;
    for n in 0..ROCKETS {
        let (sin, cos) = (spin + n as f32 / ROCKETS as f32 * TAU).sin_cos();
        let at = |out: f32| [cos * out, bob, sin * out];
        model.tube(tube(at(0.1), at(0.3), 0.028).sides(6), ROCKET);
        model.tube(tube(at(0.3), at(0.37), 0.028).taper(0.0).sides(6), team);
    }
}

/// An Eye in the Sky: a broad dome hanging in the air on four rotors, with a red eye at its
/// front and six heavy bombs in a ring under it.
fn eye_in_the_sky(model: &mut Model, time: f32, team: Paint) {
    const BOMBS: usize = 6;
    const RADIUS: f32 = 0.42;
    model.ball([0.0, 0.0, 0.0], [RADIUS, 0.17, RADIUS], true, STEEL);
    let sides = model.sides(16);
    model.tube(
        tube([0.0, -0.07, 0.0], [0.0, 0.0, 0.0], RADIUS).sides(sides),
        GUNMETAL,
    );
    model.tube(
        tube([0.0, -0.01, 0.0], [0.0, 0.03, 0.0], RADIUS + 0.01).sides(sides),
        team,
    );
    model.ball([RADIUS - 0.04, 0.03, 0.0], [0.07; 3], false, LASER_CRYSTAL);
    let spin = time * 25.0;
    for (along, across) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        let hub = [along * 0.5, 0.06, across * 0.5];
        model.bar([along * 0.25, 0.04, across * 0.25], hub, 0.02, GUNMETAL);
        model.tube(
            tube([hub[0], 0.02, hub[2]], [hub[0], 0.1, hub[2]], 0.04).sides(8),
            GUNMETAL,
        );
        let (sin, cos) = (spin * along * across).sin_cos();
        let blade = [cos * 0.17, 0.0, sin * 0.17];
        model.bar(
            [hub[0] - blade[0], 0.11, hub[2] - blade[2]],
            [hub[0] + blade[0], 0.11, hub[2] + blade[2]],
            0.012,
            STEEL,
        );
    }
    for n in 0..BOMBS {
        let (sin, cos) = (n as f32 / BOMBS as f32 * TAU).sin_cos();
        let (a, c) = (cos * 0.24, sin * 0.24);
        model.tube(tube([a, -0.08, c], [a, -0.28, c], 0.055).sides(8), GUNMETAL);
        model.tube(
            tube([a, -0.28, c], [a, -0.36, c], 0.055)
                .taper(0.0)
                .sides(8),
            GUNMETAL,
        );
        model.block([a, -0.1, c], [0.07, 0.025, 0.006], team);
    }
}

/// A laser's crystal, glowing red.
const LASER_CRYSTAL: Paint = Paint {
    color: [1.0, 0.2, 0.15],
    shine: 0.0,
};

/// A Kobra: a hull on two tracks with a cannon longer than itself resting on top. Deployed,
/// a leg of three segments unfolds from the deck and raises the cannon high, pointing up at a
/// slant, and folds back down as it packs up.
fn siege_tank(model: &mut Model, unit: &Unit, team: Paint) {
    let track = Track {
        rear: -0.42,
        front: 0.42,
        height: 0.12,
        radius: 0.1,
        half_width: 0.07,
        road_wheels: 5,
        links: 22,
    };
    tracks(model, unit, &track, 0.33);
    model.bevel([0.0, 0.25, 0.0], [0.48, 0.1, 0.24], 0.03, STEEL);
    model.bevel([-0.15, 0.37, 0.0], [0.28, 0.03, 0.2], 0.02, team);
    if unit.popped <= 0.0 {
        // Resting on a cradle at each end.
        model.bevel([-0.35, 0.42, 0.0], [0.05, 0.05, 0.08], 0.01, GUNMETAL);
        model.bevel([0.38, 0.42, 0.0], [0.05, 0.05, 0.08], 0.01, GUNMETAL);
        let [tip, height] = units::SIEGE_TIP;
        model.tube(
            tube([-0.72, height, 0.0], [tip - 0.1, height, 0.0], 0.065),
            GUNMETAL,
        );
        model.tube(
            tube([tip - 0.1, height, 0.0], [tip, height, 0.0], 0.085),
            GUNMETAL,
        );
        return;
    }
    const FOLDED: [[f32; 3]; 4] = [
        [-0.1, 0.38, 0.0],
        [0.2, 0.4, 0.0],
        [-0.12, 0.42, 0.0],
        [0.1, 0.45, 0.0],
    ];
    const RAISED: [[f32; 3]; 4] = [
        [-0.1, 0.38, 0.0],
        [0.18, 0.85, 0.0],
        [-0.1, 1.28, 0.0],
        [0.1, 1.6, 0.0],
    ];
    let t = unit.popped;
    let ease = t * t * (3.0 - 2.0 * t);
    let joints: [[f32; 3]; 4] = std::array::from_fn(|j| {
        std::array::from_fn(|c| FOLDED[j][c] + (RAISED[j][c] - FOLDED[j][c]) * ease)
    });
    for pair in joints.windows(2) {
        model.tube(tube(pair[0], pair[1], 0.055).sides(8), STEEL);
        model.ball(pair[1], [0.07; 3], false, BRONZE);
    }
    let pivot = joints[3];
    let [tip, height] = units::siege_tip(unit.popped);
    let way = [tip - pivot[0], height - pivot[1], 0.0];
    let back = [pivot[0] - way[0] * 0.7, pivot[1] - way[1] * 0.7, 0.0];
    let brake = [pivot[0] + way[0] * 0.9, pivot[1] + way[1] * 0.9, 0.0];
    model.tube(tube(back, brake, 0.065), GUNMETAL);
    model.tube(tube(brake, [tip, height, 0.0], 0.085), GUNMETAL);
}
