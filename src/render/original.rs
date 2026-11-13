//! The original game's own models, read off its disc when it is mounted, drawn in place of
//! ours for the machines, buildings and towers it has them for. They are measured in the
//! world's own coords; each looks to -z, with its up y and its right x. Each face takes the
//! colors its picture has under it, and the house color where the picture leaves room for it.

use super::Canvas;
use super::models::{Model, Paint, Place};
use crate::archive::{self, Archive};
use crate::game::Faction;
use crate::units::{Role, Unit, UnitKind};
use crate::xbf::{self, Image, Object};
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
/// How many points of its picture a face's color is taken from on each side.
const SAMPLE_STEPS: usize = 3;
/// Less of a face than this showing through its picture and it is left out.
const LEAST_COVER: f32 = 0.5;

static SHAPES: OnceLock<HashMap<&'static str, Shape>> = OnceLock::new();

/// A face ready to draw, given as [along, up, across]: its color is `color`, plus the house
/// color times `team`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Face {
    points: [[f32; 3]; 3],
    normals: [[f32; 3]; 3],
    color: [f32; 3],
    team: f32,
}

/// A model ready to draw: its body, and what turns on top of it, given from the point it turns
/// about.
#[derive(Clone, Debug, Default, PartialEq)]
struct Shape {
    body: Vec<Face>,
    head: Vec<Face>,
    pivot: Option<[f32; 3]>,
    /// How far its head reaches down under the ground, which a turret that pops up comes up by.
    sunk: f32,
}

/// A picture as a model's faces use it: its pixels, and whether its pure blue is the house
/// color; or a light's glow, which is no surface; or one not on the disc.
enum Picture {
    Painted { image: Image, team: bool },
    Glow,
    Missing,
}

/// Starts reading the models off the disc mounted at `disc`, out of the way of everything
/// else; until they are read, and if they cannot be, ours are drawn.
pub fn load(disc: &Path) {
    let disc = disc.to_path_buf();
    std::thread::spawn(move || {
        if let Some(shapes) = read_shapes(&disc) {
            let _ = SHAPES.set(shapes);
        }
    });
}

fn read_shapes(disc: &Path) -> Option<HashMap<&'static str, Shape>> {
    let index = archive::from_cabinet(&disc.join(CABINET), INDEX)?;
    let archive = Archive::open(&index, &disc.join(DATA))?;
    let mut pictures = HashMap::new();
    let names = MODELS.iter().map(|&(_, _, name)| name);
    let names = names.chain(CONSTRUCTION_YARDS.iter().map(|&(_, name)| name));
    let shapes: HashMap<_, _> = names
        .filter_map(|name| Some((name, read_shape(&archive, name, &mut pictures)?)))
        .collect();
    (!shapes.is_empty()).then_some(shapes)
}

fn read_shape(
    archive: &Archive,
    name: &str,
    pictures: &mut HashMap<String, Picture>,
) -> Option<Shape> {
    let model = xbf::parse(&archive.read(name)?)?;
    for texture in &model.textures {
        if !pictures.contains_key(texture) {
            pictures.insert(texture.clone(), read_picture(archive, texture));
        }
    }
    let used: Vec<&Picture> = model
        .textures
        .iter()
        .map(|texture| &pictures[texture])
        .collect();
    let mut shape = Shape::default();
    let identity = std::array::from_fn(|r| std::array::from_fn(|c| f32::from(r == c)));
    for object in &model.objects {
        gather(object, &identity, false, &used, &mut shape);
    }
    Some(shape)
}

/// The picture of this name, its marks in front of it saying how it is used.
fn read_picture(archive: &Archive, name: &str) -> Picture {
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

/// Adds an object's faces and those of what is in it to the shape, placed by its parents'
/// transforms `parent`, into its head once the part that turns has been come to.
fn gather(
    object: &Object,
    parent: &[[f32; 4]; 4],
    in_head: bool,
    pictures: &[&Picture],
    shape: &mut Shape,
) {
    let world: [[f32; 4]; 4] = std::array::from_fn(|r| {
        std::array::from_fn(|c| (0..4).map(|k| object.transform[r][k] * parent[k][c]).sum())
    });
    let turns_here = !in_head && shape.pivot.is_none() && object.name.contains("::0");
    if turns_here {
        shape.pivot = Some(to_place([world[3][0], world[3][1], world[3][2]]));
    }
    let in_head = in_head || turns_here;
    if !is_marker(&object.name) {
        for face in &object.faces {
            let picture = face.texture.and_then(|n| pictures.get(n).copied());
            let Some((color, team)) = paint(face, picture) else {
                continue;
            };
            let corner = |n: usize| object.vertices[face.corners[n]];
            let points = std::array::from_fn(|n| to_place(point(&world, corner(n).position)));
            let normals = std::array::from_fn(|n| to_place(normal(&world, corner(n).normal)));
            let mut face = Face {
                points,
                normals,
                color,
                team,
            };
            if in_head {
                for &[_, up, _] in &face.points {
                    shape.sunk = shape.sunk.max(-up);
                }
                let pivot = shape.pivot.unwrap_or_default();
                face.points = face
                    .points
                    .map(|p| std::array::from_fn(|i| p[i] - pivot[i]));
                shape.head.push(face);
            } else {
                shape.body.push(face);
            }
        }
    }
    for child in &object.children {
        gather(child, &world, in_head, pictures, shape);
    }
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
        Some(Picture::Painted { image, team }) => (image, *team),
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
    SHAPES.get()?.get(name)
}

/// Draws the original's model of the unit in its house color, if there is one to draw.
pub(super) fn draw(canvas: &mut Canvas, unit: &Unit, team: [f32; 3]) -> bool {
    let Some(shape) = shape_of(unit) else {
        return false;
    };
    let (body, head) = match unit.kind {
        UnitKind::Aircraft => {
            let body = Place::flying(unit);
            (body, body.moved(shape.pivot.unwrap_or_default()))
        }
        // Lined up with the map's grid squares, the front of it out of the side it faces.
        UnitKind::Building => {
            let body = Place::turned([unit.x, unit.y, unit.z], unit.facing.turn() + FRAC_PI_2);
            (body, body.moved(shape.pivot.unwrap_or_default()))
        }
        // The head turns on its own, and one that pops up comes up out of the ground.
        UnitKind::Tower => {
            let body = Place::turned([unit.x, unit.y, unit.z], unit.facing.turn() + FRAC_PI_2);
            let mut pivot = body.point(shape.pivot.unwrap_or_default());
            if unit.role.pops_up() {
                pivot[1] += unit.popped * shape.sunk;
            }
            (body, Place::turned(pivot, unit.heading))
        }
        UnitKind::Vehicle | UnitKind::Infantry => {
            let body = Place::level(unit);
            (body, body.moved(shape.pivot.unwrap_or_default()))
        }
    };
    make(&mut Model::new(canvas, body), &shape.body, team);
    make(&mut Model::new(canvas, head), &shape.head, team);
    true
}

fn make(model: &mut Model, faces: &[Face], team: [f32; 3]) {
    for face in faces {
        let color = std::array::from_fn(|i| face.color[i] + face.team * team[i]);
        model.triangle(
            face.points,
            face.normals,
            Paint {
                color,
                shine: SHINE,
            },
        );
    }
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
        let house = Picture::Painted { image, team: true };
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
    fn a_part_inside_a_marker_still_shows_but_the_marker_does_not() {
        assert!(is_marker("#AT_trike_parent"));
        assert!(is_marker("SLCTattrike"));
        assert!(is_marker("?Spotlight"));
        assert!(!is_marker("::0rot"));
        assert!(!is_marker("~~0Body"));
    }
}
