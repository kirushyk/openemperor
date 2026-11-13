//! The original game's own models, read off its disc when it is mounted, drawn in place of
//! ours for the machines, buildings and towers it has them for. They are measured in the
//! world's own coords; each looks to -z, with its up y and its right x. Each face takes the
//! colors its picture has under it, and the house color where the picture leaves room for it.

use super::Canvas;
use super::models::{Model, Paint, Place};
use crate::archive::{self, Archive};
use crate::game::Faction;
use crate::platform::NO_PICTURE;
use crate::units::{Harvest, Role, Unit, UnitKind};
use crate::weapons::REVEAL_SECONDS;
use crate::xbf::{self, Animation, Image, Object, Sequence};
use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;
use std::path::Path;
use std::sync::OnceLock;

/// Where on the disc the index of the models' data file is, and the data file itself.
const CABINET: &str = "INSTALL/Game1.cab";
const INDEX: &str = "\\data\\3DDATA0001.RFH";
const DATA: &str = "INSTALL/DATA/3DDATA0001.RFD";
const TEXTURES: &str = "Textures\\";

/// The model of each role, for every house or for one, out of the data file.
const MODELS: &[(Role, Option<Faction>, &str)] = {
    use Faction::{Atreides, Harkonnen, Ordos};
    &[
        (Role::Tank, None, "Units\\HK_assault_H0.xbf"),
        (Role::RepairVehicle, None, "Units\\AT_Repair_H0.xbf"),
        (Role::Mcv, None, "Units\\G_MCV_h0.xbf"),
        (Role::Harvester, None, "Units\\G_harvester_h0.XbF"),
        (Role::Catapult, None, "Units\\HK_Inkvine_H0.xbf"),
        (Role::RocketLauncher, None, "Units\\HK_missile_H0.xbf"),
        (Role::Mongoose, None, "Units\\AT_mongoose_H0.xbf"),
        (Role::Minotaurus, None, "Units\\AT_minotaurus_H0.xbf"),
        (Role::AdvancedCarryall, None, "Units\\AT_Carryall_H0.xbf"),
        (Role::Carryall, None, "Units\\G_Carryall_H0.XbF"),
        (Role::AirDefensePlatform, None, "Units\\HK_airgun_H0.xbf"),
        (Role::AirDrone, None, "Units\\AT_drone_H0.xbf"),
        (Role::Ornithopter, None, "Units\\AT_Ornithopter_H0.xbf"),
        (Role::Destroyer, None, "Units\\HK_Ornithopter_H0.xbf"),
        (Role::AirMine, None, "Units\\OR_ADD_H0.xbf"),
        (Role::EyeInTheSky, None, "Units\\OR_EITS_H0.xbf"),
        (Role::SandBike, None, "Units\\AT_Trike_H0.xbf"),
        (Role::DiskSaw, None, "Units\\HK_buzzsaw_H0.xbf"),
        (Role::FlameTank, None, "Units\\HK_flame_H0.xbf"),
        (Role::Devastator, None, "Units\\HK_devastator_H0.xbf"),
        (Role::Recon, None, "Units\\Or_DustScout_H0.xbf"),
        (Role::LaserTank, None, "Units\\OR_Lasertank_H0.XBF"),
        (Role::Kobra, None, "Units\\OR_Kobra_H0.XBF"),
        (Role::RocketApc, None, "Units\\Or_apc_H0.xbf"),
        (Role::StealthApc, None, "Units\\AT_APC_H0.xbf"),
        (Role::Tower, None, "Buildings\\AT_rocketturret_h0.XbF"),
        (Role::GunPost, None, "Buildings\\AT_MGT_H0.xbf"),
        (Role::GunTower, None, "Buildings\\HK_GunTurret_H0.XBF"),
        (Role::FlameTower, None, "Buildings\\HK_flameturret_h0.XbF"),
        (Role::GasTower, None, "Buildings\\OR_gasturret_H0.XbF"),
        (Role::LaserTower, None, "Buildings\\OR_popupturret_H0.XBF"),
        (
            Role::Outpost,
            Some(Atreides),
            "Buildings\\AT_OUTPOST_H0.XbF",
        ),
        (
            Role::Outpost,
            Some(Harkonnen),
            "Buildings\\hk_Outpost_H0.XBF",
        ),
        (Role::Outpost, Some(Ordos), "Buildings\\OR_Outpost_h0.XbF"),
        (
            Role::Barracks,
            Some(Atreides),
            "Buildings\\at_barracks_H0.xbf",
        ),
        (
            Role::Barracks,
            Some(Harkonnen),
            "Buildings\\HK_barracks_h0.XbF",
        ),
        (Role::Barracks, Some(Ordos), "Buildings\\OR_Barracks_H0.XBF"),
        (
            Role::Factory,
            Some(Atreides),
            "Buildings\\at_factory_H0.xbf",
        ),
        (
            Role::Factory,
            Some(Harkonnen),
            "Buildings\\HK_Factory_H0.xbf",
        ),
        (Role::Factory, Some(Ordos), "Buildings\\or_factory_H0.XBF"),
        (Role::Hangar, Some(Atreides), "Buildings\\at_hanger_H0.xbf"),
        (Role::Hangar, Some(Harkonnen), "Buildings\\hk_hanger_H0.xbf"),
        (Role::Hangar, Some(Ordos), "Buildings\\OR_Hanger_H0.xbf"),
        (
            Role::LandingPad,
            Some(Atreides),
            "Buildings\\at_helipad_H0.XbF",
        ),
        (
            Role::LandingPad,
            Some(Harkonnen),
            "Buildings\\hk_helipad_H0.XbF",
        ),
        (
            Role::Refinery,
            Some(Atreides),
            "Buildings\\at_refinery_h0.xbf",
        ),
        (
            Role::Refinery,
            Some(Harkonnen),
            "Buildings\\hk_refinery_H0.xbf",
        ),
        (Role::Refinery, Some(Ordos), "Buildings\\OR_refinery_H0.xbf"),
        (Role::Palace(Atreides), None, "Buildings\\AT_Palace_H0.XbF"),
        (Role::Palace(Harkonnen), None, "Buildings\\hk_palace_h0.XBF"),
        (Role::Palace(Ordos), None, "Buildings\\OR_Palace_H0.XBF"),
    ]
};

/// The construction yard each house's MCV deploys into.
const CONSTRUCTION_YARDS: [(Faction, &str); 3] = [
    (Faction::Atreides, "Buildings\\AT_conyard_H0.XbF"),
    (Faction::Harkonnen, "Buildings\\HK_conyard_H0.XbF"),
    (Faction::Ordos, "Buildings\\OR_conyard_H0.XBF"),
];

/// How the original's paint shines.
const SHINE: f32 = 0.2;
/// The color of a face with no picture, or whose picture is not on the disc.
const UNPAINTED: [f32; 3] = [0.5, 0.5, 0.5];
/// How many frames of their animations the original's models go through in a second.
const FRAMES_PER_SECOND: f32 = 25.0;
/// How many frames apart one machine's loops are from the next one's, so that they do not all
/// move together.
const FRAMES_APART: usize = 13;
/// How many points of its picture a face's color is taken from on each side.
const SAMPLE_STEPS: usize = 3;
/// Less of a face than this showing through its picture and it is left out.
const LEAST_COVER: f32 = 0.5;

const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// How wide each picture is made for the backend, whatever its size on the disc.
pub const PICTURE_SIDE: usize = 128;

static ORIGINAL: OnceLock<Original> = OnceLock::new();

/// The models read off the disc, and the pictures they are painted with, one after another.
struct Original {
    shapes: HashMap<&'static str, Shape>,
    pictures: Vec<u8>,
}

/// A face ready to draw, given in its part's own room: painted with the picture `picture` if
/// it has one, each corner at `uv` on it; otherwise, and for a backend that does not paint
/// with pictures, its color is `color` plus the house color times `team`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Face {
    points: [[f32; 3]; 3],
    normals: [[f32; 3]; 3],
    color: [f32; 3],
    team: f32,
    picture: Option<usize>,
    uv: [[f32; 2]; 3],
}

/// A model ready to draw: its parts, each after the one it sits in, the point its head turns
/// about, and the runs of its animation.
#[derive(Clone, Debug, Default, PartialEq)]
struct Shape {
    parts: Vec<Part>,
    /// Where its head turns about, as [along, up, across], where its first frame has it.
    pivot: Option<[f32; 3]>,
    /// How far its head reaches down under the ground, which a turret that pops up comes up by.
    sunk: f32,
    sequences: Vec<Sequence>,
}

/// A part of a model: its faces, the part it sits in, where it is in that one, and whether it
/// is in the head.
#[derive(Clone, Debug, Default, PartialEq)]
struct Part {
    faces: Vec<Face>,
    parent: Option<usize>,
    transform: [[f32; 4]; 4],
    animation: Option<Animation>,
    head: bool,
}

impl Part {
    /// Where it is in its parent at a frame of the animation.
    fn at(&self, frame: usize) -> [[f32; 4]; 4] {
        self.animation
            .as_ref()
            .and_then(|animation| animation.at(frame))
            .copied()
            .unwrap_or(self.transform)
    }
}

/// A picture as a model's faces use it: its pixels, whether its pure blue is the house color,
/// and which it is of those given to the backend; or a light's glow, which is no surface; or
/// one not on the disc.
enum Picture {
    Painted {
        image: Image,
        team: bool,
        layer: usize,
    },
    Glow,
    Missing,
}

/// Starts reading the models off the disc mounted at `disc`, out of the way of everything
/// else; until they are read, and if they cannot be, ours are drawn.
pub fn load(disc: &Path) {
    let disc = disc.to_path_buf();
    std::thread::spawn(move || {
        if let Some(original) = read_shapes(&disc) {
            let _ = ORIGINAL.set(original);
        }
    });
}

/// The pictures the original's models are painted with, once they are read: squares
/// `PICTURE_SIDE` wide, one after another, as the backend takes them.
pub fn pictures() -> Option<&'static [u8]> {
    Some(&ORIGINAL.get()?.pictures)
}

fn read_shapes(disc: &Path) -> Option<Original> {
    let index = archive::from_cabinet(&disc.join(CABINET), INDEX)?;
    let archive = Archive::open(&index, &disc.join(DATA))?;
    let mut pictures = HashMap::new();
    let names = MODELS.iter().map(|&(_, _, name)| name);
    let names = names.chain(CONSTRUCTION_YARDS.iter().map(|&(_, name)| name));
    let shapes: HashMap<_, _> = names
        .filter_map(|name| Some((name, read_shape(&archive, name, &mut pictures)?)))
        .collect();
    let mut painted: Vec<(usize, &Image, bool)> = pictures
        .values()
        .filter_map(|picture| match picture {
            Picture::Painted { image, team, layer } => Some((*layer, image, *team)),
            _ => None,
        })
        .collect();
    painted.sort_by_key(|&(layer, _, _)| layer);
    let pictures = painted
        .into_iter()
        .flat_map(|(_, image, team)| square(image, team))
        .collect();
    (!shapes.is_empty()).then_some(Original { shapes, pictures })
}

/// A picture as the backend takes it: made `PICTURE_SIDE` wide by taking the nearest pixel,
/// its see-through magenta a hole of alpha 0, and its house parts black with how much house
/// color shows there in an alpha from 255 down to 128.
fn square(image: &Image, team: bool) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(PICTURE_SIDE * PICTURE_SIDE * 4);
    for y in 0..PICTURE_SIDE {
        for x in 0..PICTURE_SIDE {
            let [r, g, b, a] = image.pixels
                [y * image.height / PICTURE_SIDE * image.width + x * image.width / PICTURE_SIDE];
            let texel = if a < 128 || (r > 229 && g < 26 && b > 229) {
                [0; 4]
            } else if team && r < 10 && g < 10 && b > 15 {
                [0, 0, 0, 255 - (u16::from(b) * 127 / 255) as u8]
            } else {
                [r, g, b, 255]
            };
            bytes.extend(texel);
        }
    }
    bytes
}

fn read_shape(
    archive: &Archive,
    name: &str,
    pictures: &mut HashMap<String, Picture>,
) -> Option<Shape> {
    let model = xbf::parse(&archive.read(name)?)?;
    for texture in &model.textures {
        if !pictures.contains_key(texture) {
            let layer = pictures
                .values()
                .filter(|picture| matches!(picture, Picture::Painted { .. }))
                .count();
            pictures.insert(texture.clone(), read_picture(archive, texture, layer));
        }
    }
    let used: Vec<&Picture> = model
        .textures
        .iter()
        .map(|texture| &pictures[texture])
        .collect();
    let mut shape = Shape {
        sequences: model.sequences,
        ..Shape::default()
    };
    for object in &model.objects {
        gather(object, None, &IDENTITY, false, &used, &mut shape);
    }
    Some(shape)
}

/// The picture of this name, its marks in front of it saying how it is used, to be the
/// backend's `layer`th if it is painted.
fn read_picture(archive: &Archive, name: &str, layer: usize) -> Picture {
    let marks = &name[..name.len()
        - name
            .trim_start_matches(|c: char| !c.is_alphanumeric())
            .len()];
    if marks.contains('!') {
        return Picture::Glow;
    }
    match archive
        .read(&format!("{TEXTURES}{name}"))
        .and_then(|data| xbf::tga(&data))
    {
        Some(image) => Picture::Painted {
            image,
            team: marks.contains('='),
            layer,
        },
        None => Picture::Missing,
    }
}

/// Whether an object is only there to mark something for the game: a place, a light, a shadow,
/// a selection box or a muzzle, not a part to see. What is inside it can still be.
fn is_marker(name: &str) -> bool {
    name.starts_with(['?', '#', '>', '%', '{'])
        || name.starts_with("SLCT")
        || name.to_ascii_lowercase().starts_with("shadow")
}

/// Adds an object and what is in it to the shape as parts, in its `parent`, placed there by
/// its parents' transforms at the first frame, `above`; into its head once the part that turns
/// has been come to.
fn gather(
    object: &Object,
    parent: Option<usize>,
    above: &[[f32; 4]; 4],
    in_head: bool,
    pictures: &[&Picture],
    shape: &mut Shape,
) {
    let world = times(&object.transform, above);
    let turns_here = !in_head && shape.pivot.is_none() && object.name.contains("::0");
    if turns_here {
        shape.pivot = Some(to_place([world[3][0], world[3][1], world[3][2]]));
    }
    let head = in_head || turns_here;
    let mut faces = Vec::new();
    if !is_marker(&object.name) {
        for face in &object.faces {
            let picture = face.texture.and_then(|n| pictures.get(n).copied());
            let Some((color, team)) = paint(face, picture) else {
                continue;
            };
            let corner = |n: usize| object.vertices[face.corners[n]];
            let face = Face {
                points: std::array::from_fn(|n| corner(n).position),
                normals: std::array::from_fn(|n| corner(n).normal),
                color,
                team,
                picture: match picture {
                    Some(Picture::Painted { layer, .. }) => Some(*layer),
                    _ => None,
                },
                uv: face.uv,
            };
            if head {
                for &position in &face.points {
                    shape.sunk = shape.sunk.max(-point(&world, position)[1]);
                }
            }
            faces.push(face);
        }
    }
    shape.parts.push(Part {
        faces,
        parent,
        transform: object.transform,
        animation: object.animation.clone(),
        head,
    });
    let this = shape.parts.len() - 1;
    for child in &object.children {
        gather(child, Some(this), &world, head, pictures, shape);
    }
}

/// One transform after another, for row vectors: `first`, then `then`.
fn times(first: &[[f32; 4]; 4], then: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..4).map(|k| first[r][k] * then[k][c]).sum()))
}

fn point(world: &[[f32; 4]; 4], [x, y, z]: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|c| x * world[0][c] + y * world[1][c] + z * world[2][c] + world[3][c])
}

fn normal(world: &[[f32; 4]; 4], [x, y, z]: [f32; 3]) -> [f32; 3] {
    let way: [f32; 3] =
        std::array::from_fn(|c| x * world[0][c] + y * world[1][c] + z * world[2][c]);
    let length = way.iter().map(|c| c * c).sum::<f32>().sqrt();
    if length > 0.0 {
        way.map(|c| c / length)
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// A point of the original's model as [along, up, across].
fn to_place([x, y, z]: [f32; 3]) -> [f32; 3] {
    [-z, y, x]
}

/// A face's color and its share of house color, from points spread over its picture under
/// it; `None` if the picture is a glow, or mostly shows what is behind it.
fn paint(face: &xbf::Face, picture: Option<&Picture>) -> Option<([f32; 3], f32)> {
    let (image, house) = match picture {
        Some(Picture::Painted { image, team, .. }) => (image, *team),
        Some(Picture::Glow) => return None,
        Some(Picture::Missing) | None => return Some((UNPAINTED, 0.0)),
    };
    let (mut color, mut team, mut cover, mut samples) = ([0.0; 3], 0.0, 0.0, 0.0);
    for i in 0..=SAMPLE_STEPS {
        for j in 0..=SAMPLE_STEPS - i {
            let weights = [i, j, SAMPLE_STEPS - i - j].map(|n| n as f32 / SAMPLE_STEPS as f32);
            let uv = std::array::from_fn(|c| (0..3).map(|n| weights[n] * face.uv[n][c]).sum());
            let [r, g, b, a] = image.at(uv).map(|c| f32::from(c) / 255.0);
            // Magenta is where a picture is see-through.
            let a = if r > 0.9 && g < 0.1 && b > 0.9 {
                0.0
            } else {
                a
            };
            samples += 1.0;
            cover += a;
            if house && r < 0.04 && g < 0.04 && b > 0.06 {
                team += a * b;
            } else {
                color = [0, 1, 2].map(|n| color[n] + a * [r, g, b][n]);
            }
        }
    }
    if cover < LEAST_COVER * samples {
        return None;
    }
    Some((color.map(|c| c / cover), team / cover))
}

/// The model the original has for the unit, if it has been read.
fn shape_of(unit: &Unit) -> Option<&'static Shape> {
    let name = if unit.role == Role::Mcv && unit.deployed {
        CONSTRUCTION_YARDS
            .iter()
            .find(|&&(house, _)| house == unit.house)?
            .1
    } else {
        MODELS
            .iter()
            .find(|&&(role, house, _)| role == unit.role && house.is_none_or(|h| h == unit.house))?
            .2
    };
    ORIGINAL.get()?.shapes.get(name)
}

/// Which frame of its model's animation a machine is at, by what it is doing and the time
/// of the battle: its gun going up, a shot it has just fired, or one of its loops. A building
/// or a tower stays at the first.
fn frame(shape: &Shape, unit: &Unit, time: f32) -> usize {
    let find = |name: &str| {
        shape
            .sequences
            .iter()
            .find(|sequence| sequence.name == name)
            .map(|sequence| sequence.ranges.as_slice())
    };
    if !matches!(unit.kind, UnitKind::Vehicle | UnitKind::Aircraft) {
        return 0;
    }
    if unit.role == Role::Kobra
        && unit.popped > 0.0
        && let Some(&[(first, last), ..]) = find("Deploy Gun")
    {
        return first + ((last - first) as f32 * unit.popped.min(1.0)) as usize;
    }
    let played = (time * FRAMES_PER_SECOND) as usize;
    if unit.revealed > 0.0
        && let Some(frame) = find("Fire 0").and_then(|ranges| {
            once(
                ranges,
                ((REVEAL_SECONDS - unit.revealed) * FRAMES_PER_SECOND) as usize,
            )
        })
    {
        return frame;
    }
    let moving = unit.is_moving();
    let names: &[&str] = match unit.harvest {
        _ if unit.kind == UnitKind::Aircraft && unit.altitude > 0.0 && moving => &["Fly", "Move"],
        _ if unit.kind == UnitKind::Aircraft && unit.altitude > 0.0 => &["Hover", "Stationary"],
        Harvest::Collecting if unit.role == Role::Harvester => &["Harv Eat Hold"],
        Harvest::Dumping(_) if unit.role == Role::Harvester => &["Harv Unload Hold"],
        _ if moving => &["Move"],
        _ => &["Stationary"],
    };
    names
        .iter()
        .find_map(|&name| find(name))
        .map_or(0, |ranges| {
            looped(ranges, played + unit.id as usize * FRAMES_APART)
        })
}

/// How many frames a range plays: one at least, for one that holds a single frame.
fn length((first, last): (usize, usize)) -> usize {
    (last - first).max(1)
}

/// The frame `played` frames into ranges played one after another, or `None` once past them.
fn once(ranges: &[(usize, usize)], mut played: usize) -> Option<usize> {
    for &range in ranges {
        if played < length(range) {
            return Some(range.0 + played);
        }
        played -= length(range);
    }
    None
}

/// The frame `played` frames into ranges played one after another, over and over.
fn looped(ranges: &[(usize, usize)], played: usize) -> usize {
    let total: usize = ranges.iter().copied().map(length).sum();
    once(ranges, played % total.max(1)).unwrap_or(0)
}

/// Draws the original's model of the unit in its house color, as it is at this time of the
/// battle, if there is one to draw.
pub(super) fn draw(canvas: &mut Canvas, unit: &Unit, team: [f32; 3], time: f32) -> bool {
    let Some(shape) = shape_of(unit) else {
        return false;
    };
    let pivot = shape.pivot.unwrap_or_default();
    let (body, head) = match unit.kind {
        UnitKind::Aircraft => {
            let body = Place::flying(unit);
            (body, body.moved(pivot))
        }
        // Lined up with the map's grid squares, the front of it out of the side it faces.
        UnitKind::Building => {
            let body = Place::turned([unit.x, unit.y, unit.z], unit.facing.turn() + FRAC_PI_2);
            (body, body.moved(pivot))
        }
        // The head turns on its own, and one that pops up comes up out of the ground.
        UnitKind::Tower => {
            let body = Place::turned([unit.x, unit.y, unit.z], unit.facing.turn() + FRAC_PI_2);
            let mut point = body.point(pivot);
            if unit.role.pops_up() {
                point[1] += unit.popped * shape.sunk;
            }
            (body, Place::turned(point, unit.heading))
        }
        UnitKind::Vehicle | UnitKind::Infantry => {
            let body = Place::level(unit);
            (body, body.moved(pivot))
        }
    };
    let frame = frame(shape, unit, time);
    let mut worlds: Vec<[[f32; 4]; 4]> = Vec::with_capacity(shape.parts.len());
    let mut model = Model::new(canvas, body);
    for part in &shape.parts {
        let above = part.parent.map_or(&IDENTITY, |parent| &worlds[parent]);
        let world = times(&part.at(frame), above);
        let (mut model, from) = if part.head {
            (model.at(head), pivot)
        } else {
            (model.at(body), [0.0; 3])
        };
        for face in &part.faces {
            let color = std::array::from_fn(|i| face.color[i] + face.team * team[i]);
            let points = face.points.map(|p| {
                let place = to_place(point(&world, p));
                std::array::from_fn(|i| place[i] - from[i])
            });
            let normals = face.normals.map(|n| to_place(normal(&world, n)));
            let picture = face.picture.map_or([NO_PICTURE; 3], |layer| {
                face.uv.map(|[u, v]| [u, v, layer as f32])
            });
            model.pictured(
                points,
                normals,
                Paint {
                    color,
                    shine: SHINE,
                },
                picture,
                team,
            );
        }
        worlds.push(world);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nose_of_an_original_model_points_along_and_its_right_across() {
        assert_eq!(to_place([0.0, 0.0, -1.0]), [1.0, 0.0, 0.0]);
        assert_eq!(to_place([1.0, 2.0, 0.0]), [0.0, 2.0, 1.0]);
    }

    #[test]
    fn pure_blue_on_a_house_picture_takes_the_house_color() {
        let image = Image {
            width: 2,
            height: 1,
            pixels: vec![[0, 0, 200, 255], [100, 50, 0, 255]],
        };
        let face = |u: f32| xbf::Face {
            corners: [0, 1, 2],
            texture: Some(0),
            uv: [[u, 0.5]; 3],
        };
        let house = Picture::Painted {
            image,
            team: true,
            layer: 0,
        };
        let near = |a: f32, b: f32| (a - b).abs() < 1e-5;
        let (color, team) = paint(&face(0.25), Some(&house)).unwrap();
        assert!(color == [0.0; 3] && near(team, 200.0 / 255.0));
        let (color, team) = paint(&face(0.75), Some(&house)).unwrap();
        assert!(near(color[0], 100.0 / 255.0) && near(color[1], 50.0 / 255.0) && team == 0.0);
        assert_eq!(paint(&face(0.25), Some(&Picture::Glow)), None);
        assert_eq!(
            paint(&face(0.25), Some(&Picture::Missing)),
            Some((UNPAINTED, 0.0))
        );
    }

    #[test]
    fn a_sequence_plays_its_ranges_one_after_another() {
        let ranges = [(12, 15), (20, 22)];
        let frames: Vec<usize> = (0..7).map(|n| looped(&ranges, n)).collect();
        assert_eq!(frames, [12, 13, 14, 20, 21, 12, 13]);
        assert_eq!(once(&ranges, 4), Some(21));
        assert_eq!(once(&ranges, 5), None);
        assert_eq!(looped(&[(160, 160)], 7), 160);
    }

    #[test]
    fn a_machine_moves_fires_and_harvests_by_its_sequences() {
        let sequence = |name: &str, first, last| Sequence {
            name: name.into(),
            ranges: vec![(first, last)],
        };
        let shape = Shape {
            sequences: vec![
                sequence("Stationary", 100, 110),
                sequence("Move", 20, 40),
                sequence("Fire 0", 60, 70),
                sequence("Harv Eat Hold", 84, 97),
            ],
            ..Shape::default()
        };
        let mut unit = Unit::for_test(0.0, 0.0, UnitKind::Vehicle);
        (unit.role, unit.id) = (Role::Harvester, 0);
        unit.destination = (unit.x, unit.z);
        assert_eq!(frame(&shape, &unit, 0.2), 105);
        unit.destination = (unit.x + 500.0, unit.z);
        assert_eq!(frame(&shape, &unit, 0.2), 25);
        unit.revealed = REVEAL_SECONDS - 0.2;
        assert_eq!(frame(&shape, &unit, 0.2), 65);
        unit.revealed = 0.0;
        unit.harvest = Harvest::Collecting;
        assert_eq!(frame(&shape, &unit, 0.2), 89);
        unit.kind = UnitKind::Building;
        assert_eq!(frame(&shape, &unit, 0.2), 0);
    }

    #[test]
    fn a_picture_is_squared_with_its_holes_and_house_parts_in_its_alpha() {
        let image = Image {
            width: 2,
            height: 1,
            pixels: vec![[255, 0, 255, 255], [0, 0, 255, 255]],
        };
        let texel = |bytes: &[u8], x: usize, y: usize| {
            let at = (y * PICTURE_SIDE + x) * 4;
            bytes[at..at + 4].to_vec()
        };
        let house = square(&image, true);
        assert_eq!(house.len(), PICTURE_SIDE * PICTURE_SIDE * 4);
        assert_eq!(texel(&house, 0, 0), [0, 0, 0, 0], "magenta is a hole");
        assert_eq!(
            texel(&house, PICTURE_SIDE - 1, 9),
            [0, 0, 0, 128],
            "all house color"
        );
        let plain = square(&image, false);
        assert_eq!(texel(&plain, PICTURE_SIDE - 1, 0), [0, 0, 255, 255]);
    }

    #[test]
    fn a_part_inside_a_marker_still_shows_but_the_marker_does_not() {
        assert!(is_marker("#AT_trike_parent"));
        assert!(is_marker("SLCTattrike"));
        assert!(is_marker("?Spotlight"));
        assert!(!is_marker("::0rot"));
        assert!(!is_marker("~~0Body"));
    }
}
