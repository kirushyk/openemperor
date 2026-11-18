//! The original's main menu scene, `frontend\main.xbf` on its disc: the ring frame, the title,
//! Arrakis under its clouds, glows, nebulas and horizons, seen from where the original's
//! camera stands. Read once with the disc's models, and drawn flat behind the menus' buttons.

use super::original::{IDENTITY, PICTURE_SIDE, point, times};
use crate::archive::Archive;
use crate::platform::Vertex;
use crate::xbf::{self, Animation, Image, Object};

/// Where the scene is in the disc's interface archive.
pub(super) const SCENE: &str = "FRONTEND\\MAIN.XBF";
/// The folder of the pictures its faces show, in the models' archive.
const TEXTURES: &str = "Textures\\";
/// How many frames of its animation go by each second, as the original's models'.
const FRAMES_PER_SECOND: f32 = 25.0;
/// How far up and down the original's screen reaches at a depth of 1: its ring frame stands
/// 300 tall each way at a depth of 525.
const HALF_DOWN: f32 = 300.0 / 525.0;
/// Nothing nearer the eye than this is drawn.
const NEAREST: f32 = 1.0;
/// The most tiles to a side a picture is cut into: one of 512 pixels into four.
const MOST_TILES: usize = 4;
/// Twice the area, in pixels, under which a piece of a face is not worth drawing.
const SLIVER: f32 = 1e-3;
/// How many times as tall as it is deep a scene drawn flat stands, to be seen from its front.
const STANDING: f32 = 2.0;
/// What starts the name of an object that only marks a place.
const MARKER: char = '#';
/// The storm clouds over Arrakis, which come out as blocks with what is read of them: left
/// out until that is understood.
const CLOUDS: &str = "cloud";
/// What in front of a picture's name says it is light, added to what is behind it.
const LIGHT: [char; 2] = ['%', '!'];

/// A picture of the scene as the backend has it: cut into `tiles` squares a side, each one of
/// the backend's pictures, the first of them its `first`th, row by row from the top left.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Picture {
    first: usize,
    tiles: usize,
    light: bool,
}

/// A triangle of the scene: its corners, where each is on its picture, and which picture.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Face {
    corners: [[f32; 3]; 3],
    uv: [[f32; 2]; 3],
    picture: usize,
}

/// A part of the scene that moves: its faces as it is made, how it stands at each frame, and
/// where what it sits in stands.
#[derive(Clone, Debug, PartialEq)]
struct Moving {
    faces: Vec<Face>,
    animation: Animation,
    above: [[f32; 4]; 4],
}

/// The scene, ready to draw.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Scene {
    pictures: Vec<Option<Picture>>,
    /// What never moves, where it stands.
    still: Vec<Face>,
    moving: Vec<Moving>,
}

/// Reads the scene out of its model, adding its pictures to those given to the backend:
/// `pictures` their bytes so far and `layers` how many they are.
pub(super) fn read(
    model: &[u8],
    textures: &Archive,
    pictures: &mut Vec<u8>,
    layers: &mut usize,
) -> Option<Scene> {
    let model = xbf::parse(model)?;
    let mut scene = Scene::default();
    for name in &model.textures {
        let image = textures
            .read(&format!("{TEXTURES}{name}"))
            .and_then(|data| xbf::tga(&data));
        scene.pictures.push(image.map(|image| {
            let tiles = (image.width / PICTURE_SIDE).clamp(1, MOST_TILES);
            let picture = Picture {
                first: *layers,
                tiles,
                light: name.starts_with(LIGHT),
            };
            for (across, down) in (0..tiles * tiles).map(|n| (n % tiles, n / tiles)) {
                pictures.extend(tile(&image, tiles, (across, down)));
            }
            *layers += tiles * tiles;
            picture
        }));
    }
    for object in &model.objects {
        gather(object, &IDENTITY, &mut scene);
    }
    (!scene.still.is_empty() || !scene.moving.is_empty()).then_some(scene)
}

/// One of the squares a picture is cut into, `PICTURE_SIDE` wide, as the backend takes it.
fn tile(image: &Image, tiles: usize, (across, down): (usize, usize)) -> Vec<u8> {
    let side = PICTURE_SIDE * tiles;
    let mut bytes = Vec::with_capacity(PICTURE_SIDE * PICTURE_SIDE * 4);
    for y in 0..PICTURE_SIDE {
        for x in 0..PICTURE_SIDE {
            let from_x = (across * PICTURE_SIDE + x) * image.width / side;
            let from_y = (down * PICTURE_SIDE + y) * image.height / side;
            bytes.extend(image.pixels[from_y * image.width + from_x]);
        }
    }
    bytes
}

/// Adds an object's faces to the scene, and those of what sits in it: where they stand if
/// nothing moves them, or else as a moving part of their own.
fn gather(object: &Object, above: &[[f32; 4]; 4], scene: &mut Scene) {
    let world = times(&affine(&object.transform), above);
    if !object.name.starts_with(MARKER) && object.name != CLOUDS {
        let faces = object.faces.iter().filter_map(|face| {
            let picture = face.texture?;
            scene.pictures.get(picture)?.as_ref()?;
            let corner = |n: usize| object.vertices.get(face.corners[n]).map(|v| v.position);
            Some(Face {
                corners: [corner(0)?, corner(1)?, corner(2)?],
                uv: face.uv,
                picture,
            })
        });
        match &object.animation {
            Some(animation) if animation.frames.len() > 1 => scene.moving.push(Moving {
                faces: faces.collect(),
                animation: animation.clone(),
                above: *above,
            }),
            _ => scene.still.extend(faces.map(|face| Face {
                corners: face.corners.map(|corner| point(&world, corner)),
                ..face
            })),
        }
    }
    for child in &object.children {
        gather(child, &world, scene);
    }
}

/// A transform as the affine one it is: some of the original's models, its cursors among
/// them, leave rubbish where its last column has nothing but a 1 at its foot.
fn affine(transform: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut fixed = *transform;
    for (row, line) in fixed.iter_mut().enumerate() {
        line[3] = if row == 3 { 1.0 } else { 0.0 };
    }
    fixed
}

/// A corner of a face on the screen, and where it is on its picture.
type Corner = ((f32, f32), [f32; 2]);

impl Scene {
    /// Draws the scene on a screen this wide and tall, as it is this many seconds on: what is
    /// farthest first. The original's 4:3 view fills the screen's height.
    pub(super) fn draw(&self, vertices: &mut Vec<Vertex>, (width, height): (f32, f32), time: f32) {
        let mut faces = self.posed(time);
        faces.retain(|face| face.corners.iter().all(|corner| corner[2] > NEAREST));
        let depth = |face: &Face| face.corners.iter().map(|c| c[2]).fold(0.0, f32::max);
        faces.sort_by(|a, b| depth(b).total_cmp(&depth(a)));
        let scale = height / 2.0 / HALF_DOWN;
        let on_screen =
            |[x, y, z]: [f32; 3]| (width / 2.0 + x / z * scale, height / 2.0 - y / z * scale);
        for face in faces {
            let Some(Some(picture)) = self.pictures.get(face.picture) else {
                continue;
            };
            let corners: [Corner; 3] =
                std::array::from_fn(|n| (on_screen(face.corners[n]), face.uv[n]));
            // Wholly off the screen to one side: not worth cutting up.
            let off = |low: bool, most: f32, of: fn(&Corner) -> f32| {
                corners
                    .iter()
                    .all(|c| if low { of(c) < 0.0 } else { of(c) > most })
            };
            if off(true, 0.0, |c| c.0.0)
                || off(false, width, |c| c.0.0)
                || off(true, 0.0, |c| c.0.1)
                || off(false, height, |c| c.0.1)
            {
                continue;
            }
            // Seen from behind: the original draws no face from its back.
            let [(a, _), (b, _), (c, _)] = corners;
            if (b.0 - a.0) * (c.1 - a.1) - (c.0 - a.0) * (b.1 - a.1) < 0.0 {
                continue;
            }
            lay(vertices, &corners, *picture);
        }
    }
}

impl Scene {
    /// Its faces where they stand this many seconds on.
    fn posed(&self, time: f32) -> Vec<Face> {
        let frame = (time * FRAMES_PER_SECOND) as usize;
        let moved = self.moving.iter().flat_map(|part| {
            let cycle = frame % part.animation.frames.len().max(1);
            let pose = part.animation.at(cycle).map_or(IDENTITY, affine);
            let world = times(&pose, &part.above);
            part.faces.iter().map(move |face| Face {
                corners: face.corners.map(|corner| point(&world, corner)),
                ..*face
            })
        });
        self.still.iter().copied().chain(moved).collect()
    }

    /// Draws the scene flat, its middle at `at` and `scale` pixels to its unit, as it is this
    /// many seconds on. The original's cursors are such scenes: most lie on the ground and are
    /// seen from straight above, what stands highest last; one that stands up, `STANDING`
    /// times as tall as it is deep, is seen from its front.
    pub(super) fn draw_flat(
        &self,
        vertices: &mut Vec<Vertex>,
        (x, y): (f32, f32),
        scale: f32,
        time: f32,
    ) {
        let mut faces = self.posed(time);
        let span = |axis: usize| {
            let along = faces.iter().flat_map(|face| face.corners.map(|c| c[axis]));
            let (low, high) = along.fold((f32::MAX, f32::MIN), |(l, h), v| (l.min(v), h.max(v)));
            high - low
        };
        let (deep, near) = if span(1) > STANDING * span(2) {
            (1, 2)
        } else {
            (2, 1)
        };
        let nearness = |face: &Face| {
            face.corners
                .iter()
                .map(|c| c[near])
                .fold(f32::MIN, f32::max)
        };
        faces.sort_by(|a, b| nearness(a).total_cmp(&nearness(b)));
        let down = if deep == 1 { -1.0 } else { 1.0 };
        for face in faces {
            let Some(Some(picture)) = self.pictures.get(face.picture) else {
                continue;
            };
            let corners: [Corner; 3] = std::array::from_fn(|n| {
                let corner = face.corners[n];
                let at = (x + corner[0] * scale, y + corner[deep] * scale * down);
                (at, face.uv[n])
            });
            lay(vertices, &corners, *picture);
        }
    }
}

/// Lays a triangle on the screen with its picture, cut along the edges of the picture's
/// tiles, each piece shown with its own tile.
fn lay(vertices: &mut Vec<Vertex>, corners: &[Corner; 3], picture: Picture) {
    let tiles = picture.tiles as f32;
    // In tiles, from a whole picture's top left: a picture repeats past its edges.
    let least = |n: usize| {
        corners
            .iter()
            .map(|c| c.1[n])
            .fold(f32::MAX, f32::min)
            .floor()
    };
    let shift = [least(0), least(1)];
    let in_tiles: Vec<Corner> = corners
        .iter()
        .map(|&(at, uv)| (at, [(uv[0] - shift[0]) * tiles, (uv[1] - shift[1]) * tiles]))
        .collect();
    let reach = |n: usize| {
        let most = in_tiles.iter().map(|c| c.1[n]).fold(0.0, f32::max);
        (most.ceil() as usize).clamp(1, picture.tiles * 4)
    };
    for down in 0..reach(1) {
        for across in 0..reach(0) {
            let (left, top) = (across as f32, down as f32);
            let mut piece = in_tiles.clone();
            for (axis, edge, keep_above) in [
                (0, left, true),
                (0, left + 1.0, false),
                (1, top, true),
                (1, top + 1.0, false),
            ] {
                piece = cut(&piece, axis, edge, keep_above);
            }
            let layer =
                picture.first + (down % picture.tiles) * picture.tiles + across % picture.tiles;
            let corner = |&(at, uv): &Corner| {
                let (u, v) = (uv[0] - left, uv[1] - top);
                if picture.light {
                    Vertex {
                        u,
                        v,
                        layer: layer as f32,
                        ..Vertex::premultiplied(at, [1.0, 1.0, 1.0, 0.0])
                    }
                } else {
                    Vertex::pictured(at, (u, v), layer)
                }
            };
            for n in 1..piece.len().saturating_sub(1) {
                let [a, b, c] = [piece[0].0, piece[n].0, piece[n + 1].0];
                // A piece that only touches the tile covers nothing.
                if ((b.0 - a.0) * (c.1 - a.1) - (c.0 - a.0) * (b.1 - a.1)).abs() > SLIVER {
                    vertices.extend([&piece[0], &piece[n], &piece[n + 1]].map(corner));
                }
            }
        }
    }
}

/// What is left of a shape on one side of a line across its picture: where its `axis`th
/// place on the picture is at least `edge`, or at most.
fn cut(shape: &[Corner], axis: usize, edge: f32, keep_above: bool) -> Vec<Corner> {
    let inside = |corner: &Corner| {
        if keep_above {
            corner.1[axis] >= edge
        } else {
            corner.1[axis] <= edge
        }
    };
    let mut kept = Vec::with_capacity(shape.len() + 1);
    for (n, from) in shape.iter().enumerate() {
        let to = &shape[(n + 1) % shape.len()];
        if inside(from) {
            kept.push(*from);
        }
        if inside(from) != inside(to) {
            let share = (edge - from.1[axis]) / (to.1[axis] - from.1[axis]);
            let between = |a: f32, b: f32| a + (b - a) * share;
            kept.push((
                (between(from.0.0, to.0.0), between(from.0.1, to.0.1)),
                [between(from.1[0], to.1[0]), between(from.1[1], to.1[1])],
            ));
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_triangle_over_four_tiles_is_cut_into_a_piece_for_each() {
        let picture = Picture {
            first: 10,
            tiles: 2,
            light: false,
        };
        // A right triangle over the whole picture's top left half.
        let corners = [
            ((0.0, 0.0), [0.0, 0.0]),
            ((100.0, 0.0), [1.0, 0.0]),
            ((0.0, 100.0), [0.0, 1.0]),
        ];
        let mut vertices = Vec::new();
        lay(&mut vertices, &corners, picture);
        let layers: std::collections::BTreeSet<usize> =
            vertices.iter().map(|v| v.layer as usize).collect();
        assert_eq!(layers.into_iter().collect::<Vec<_>>(), [10, 11, 12]);
        assert!(vertices.iter().all(|v| (0.0..=1.0).contains(&v.u)));
        assert!(vertices.iter().all(|v| (0.0..=1.0).contains(&v.v)));
        // Together the pieces cover what the triangle does.
        let area: f32 = vertices
            .chunks(3)
            .map(|t| {
                ((t[1].x - t[0].x) * (t[2].y - t[0].y) - (t[2].x - t[0].x) * (t[1].y - t[0].y))
                    .abs()
                    / 2.0
            })
            .sum();
        assert!((area - 5000.0).abs() < 1.0, "{area}");
    }

    #[test]
    fn light_adds_to_what_is_behind_it_and_a_picture_repeats_past_its_edges() {
        let glow = Picture {
            first: 3,
            tiles: 1,
            light: true,
        };
        let corners = [
            ((0.0, 0.0), [1.25, 2.0]),
            ((10.0, 0.0), [1.75, 2.0]),
            ((0.0, 10.0), [1.25, 2.5]),
        ];
        let mut vertices = Vec::new();
        lay(&mut vertices, &corners, glow);
        assert_eq!(vertices.len(), 3);
        assert!(vertices.iter().all(|v| v.a == 0.0 && v.layer == 3.0));
        assert_eq!((vertices[0].u, vertices[0].v), (0.25, 0.0));
    }

    #[test]
    fn rubbish_in_a_transforms_last_column_is_taken_as_the_affine_one_it_is() {
        let mut transform = IDENTITY;
        transform[3] = [5.0, 6.0, 7.0, -107_374_180.0];
        transform[0][3] = 3.0;
        let fixed = affine(&transform);
        assert_eq!(fixed[3], [5.0, 6.0, 7.0, 1.0]);
        assert_eq!(fixed[0][3], 0.0);
        let world = times(&fixed, &fixed);
        assert_eq!(point(&world, [0.0; 3]), [10.0, 12.0, 14.0]);
    }
}
