//! The original game's own models, read off its disc when it is mounted, drawn in place of
//! ours for the machines, buildings and towers it has them for. They are measured in the
//! world's own coords; each looks to -z, with its up y and its right x. Each face takes the
//! colors its picture has under it, and the house color where the picture leaves room for it.

use super::Canvas;
use super::models::{Model, Paint, Place};
use crate::archive::{self, Archive};
use crate::battlefield::{self, CELL_SIDE, Terrain, View};
use crate::fog::Fog;
use crate::game::Faction;
use crate::platform::{NO_PICTURE, WorldVertex};
use crate::units::{Harvest, Role, Unit, UnitKind};
use crate::weapons::REVEAL_SECONDS;
use crate::xbf::{self, Animation, Bending, Image, Object, Sequence, Slide};
use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, TAU};
use std::path::Path;
use std::sync::OnceLock;

/// Where on the disc the index of the models' data file is, and the data file itself.
const CABINET: &str = "INSTALL/Game1.cab";
const INDEX: &str = "\\data\\3DDATA0001.RFH";
const DATA: &str = "INSTALL/DATA/3DDATA0001.RFD";
const TEXTURES: &str = "Textures\\";
/// The picture the original lays its spice on the ground with, and how it lays it, as its
/// `spice.ini` and `GAME.EXE` have it.
const SPICE: &str = "spicetga_32.tga";
/// How far from its middle each corner of a patch of spice is, give or take `SPICE_REACH_JITTER`.
const SPICE_REACH: f32 = 35.0;
const SPICE_REACH_JITTER: f32 = 15.0;
/// How far a patch may lie from the middle of its cell, either way along x and z.
const SPICE_SHIFT: f32 = 127.0 / 9.0;
/// How high over the ground under its middle a patch lies.
const SPICE_LIFT: f32 = 1.0;
/// How much spice each further patch on a cell takes, past the first.
const SPICE_PER_PATCH: u8 = 64;
/// Where on its picture each corner of a patch is.
const SPICE_CORNERS: [[f32; 2]; 3] = [[0.5, 0.0], [1.0, 1.0], [0.0, 1.0]];

/// The model of each role, for every house or for one, out of the data file.
const MODELS: &[(Role, Option<Faction>, &str)] = {
    use Faction::{Atreides, Harkonnen, Ordos};
    &[
        (Role::HkAssault, None, "Units\\HK_assault_H0.xbf"),
        (Role::AtRepairUnit, None, "Units\\AT_Repair_H0.xbf"),
        (Role::Mcv, None, "Units\\G_MCV_h0.xbf"),
        (Role::Harvester, None, "Units\\G_harvester_h0.XbF"),
        (Role::HkInkVine, None, "Units\\HK_Inkvine_H0.xbf"),
        (Role::HkMissile, None, "Units\\HK_missile_H0.xbf"),
        (Role::AtMongoose, None, "Units\\AT_mongoose_H0.xbf"),
        (Role::AtMinotaurus, None, "Units\\AT_minotaurus_H0.xbf"),
        (
            Role::AdvCarryall,
            Some(Atreides),
            "Units\\AT_Carryall_H0.xbf",
        ),
        (
            Role::AdvCarryall,
            Some(Harkonnen),
            "Units\\HK_Carryall_H0.xbf",
        ),
        (Role::AdvCarryall, Some(Ordos), "Units\\OR_Carryall_H0.xbf"),
        (Role::Carryall, None, "Units\\G_Carryall_H0.XbF"),
        (Role::HkAdp, None, "Units\\HK_airgun_H0.xbf"),
        (Role::AtAdp, None, "Units\\AT_drone_H0.xbf"),
        (Role::AtOrni, None, "Units\\AT_Ornithopter_H0.xbf"),
        (Role::HkGunship, None, "Units\\HK_Ornithopter_H0.xbf"),
        (Role::OrAdp, None, "Units\\OR_ADD_H0.xbf"),
        (Role::OrEits, None, "Units\\OR_EITS_H0.xbf"),
        (Role::AtTrike, None, "Units\\AT_Trike_H0.xbf"),
        (Role::HkBuzzsaw, None, "Units\\HK_buzzsaw_H0.xbf"),
        (Role::HkFlame, None, "Units\\HK_flame_H0.xbf"),
        (Role::HkDevastator, None, "Units\\HK_devastator_H0.xbf"),
        (Role::OrDustScout, None, "Units\\Or_DustScout_H0.xbf"),
        (Role::OrLaserTank, None, "Units\\OR_Lasertank_H0.XBF"),
        (Role::OrKobra, None, "Units\\OR_Kobra_H0.XBF"),
        (Role::OrApc, None, "Units\\Or_apc_H0.xbf"),
        (Role::AtApc, None, "Units\\AT_APC_H0.xbf"),
        (Role::Scout, Some(Atreides), "Units\\AT_Scout_H0.XBF"),
        (Role::Scout, Some(Harkonnen), "Units\\HK_Scout_H0.xbf"),
        (Role::Scout, Some(Ordos), "Units\\OR_Scout_H0.xbf"),
        (Role::Engineer, Some(Atreides), "Units\\AT_Engineer_H0.XBF"),
        (Role::Engineer, Some(Harkonnen), "Units\\HK_Engineer_H0.xbf"),
        (Role::Engineer, Some(Ordos), "Units\\OR_Engineer_H0.xbf"),
        (Role::AtInfantry, None, "Units\\AT_inf_H0.xbf"),
        (Role::AtSniper, None, "Units\\AT_Sniper_H0.XBF"),
        (Role::AtKindjal, None, "Units\\AT_Kindjal_H0.xbf"),
        (Role::HkLightInf, None, "Units\\HK_ltinf_H0.xbf"),
        (Role::HkTrooper, None, "Units\\HK_Trooper_H0.xbf"),
        (Role::HkFlamer, None, "Units\\HK_Flamer_H0.xbf"),
        (Role::OrChemical, None, "Units\\OR_Chemical_H0.xbf"),
        (Role::OrSaboteur, None, "Units\\OR_Saboteur_H0.xbf"),
        (Role::OrMortar, None, "Units\\OR_Mortar_H0.xbf"),
        (
            Role::AtRocketTurret,
            None,
            "Buildings\\AT_rocketturret_h0.XbF",
        ),
        (Role::AtPillbox, None, "Buildings\\AT_MGT_H0.xbf"),
        (Role::HkGunTurret, None, "Buildings\\HK_GunTurret_H0.XBF"),
        (
            Role::HkFlameTurret,
            None,
            "Buildings\\HK_flameturret_h0.XbF",
        ),
        (Role::OrGasTurret, None, "Buildings\\OR_gasturret_H0.XbF"),
        (
            Role::OrPopUpTurret,
            None,
            "Buildings\\OR_popupturret_H0.XBF",
        ),
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
        (Role::Hanger, Some(Atreides), "Buildings\\at_hanger_H0.xbf"),
        (Role::Hanger, Some(Harkonnen), "Buildings\\hk_hanger_H0.xbf"),
        (Role::Hanger, Some(Ordos), "Buildings\\OR_Hanger_H0.xbf"),
        (
            Role::Helipad,
            Some(Atreides),
            "Buildings\\at_helipad_H0.XbF",
        ),
        (
            Role::Helipad,
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
    /// How many pictures the models have, after which a map's own come.
    layers: usize,
    archive: Archive,
    spice: Option<Spice>,
}

/// How spice is laid on the ground: its picture, which it is of those given to the backend, and
/// the original's two tables, made once with random numbers: how far from its cell's middle
/// each patch lies, and the corners of the triangles patches are, each turned its own way.
struct Spice {
    layer: usize,
    shifts: [(f32, f32); 64],
    patches: [[(f32, f32); 3]; 32],
}

impl Spice {
    fn new(layer: usize) -> Self {
        let mut random = 1;
        let mut next = || battlefield::next_random(&mut random);
        let shifts = std::array::from_fn(|_| {
            let mut shift = || (next() * 2.0 - 1.0) * SPICE_SHIFT;
            (shift(), shift())
        });
        let patches = std::array::from_fn(|_| {
            let reach = SPICE_REACH + (next() * 2.0 - 1.0) * SPICE_REACH_JITTER;
            let turn = next() * TAU;
            std::array::from_fn(|corner| {
                let angle = turn + corner as f32 * TAU / 3.0;
                (angle.sin() * reach, angle.cos() * reach)
            })
        });
        Spice {
            layer,
            shifts,
            patches,
        }
    }

    /// Lays the patches of the cell at column `u` and row `v`, as much as `seen` of it shows.
    fn lay(
        &self,
        decals: &mut Vec<WorldVertex>,
        terrain: &Terrain,
        (u, v): (usize, usize),
        seen: f32,
    ) {
        let amount = terrain.spice_in(u, v);
        if amount == 0 {
            return;
        }
        let (x, z) = battlefield::grid_to_world(u as f32 + 0.5, v as f32 + 0.5);
        let mut at = (u * 17).wrapping_sub(v * 31) & 63;
        let step = (u & v) + 1;
        for _ in 0..=amount / SPICE_PER_PATCH {
            let (dx, dz) = self.shifts[at];
            let (mx, mz) = (x + dx, z + dz);
            at = (at + step) & 63;
            let y = terrain.height_at(mx, mz) + SPICE_LIFT;
            for (&(cx, cz), [pu, pv]) in self.patches[at & 31].iter().zip(SPICE_CORNERS) {
                decals.push(WorldVertex {
                    x: mx + cx,
                    y,
                    z: mz + cz,
                    normal: [0.0, 1.0, 0.0],
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    seen,
                    level: 1.0,
                    stain: [0.0; 3],
                    picture: [pu, pv, self.layer as f32],
                    team: [0.0; 3],
                });
            }
        }
    }
}

/// A map's ground as the original shapes it: its pieces, and the pictures it is painted with,
/// to be given to the backend after the models'.
struct Land {
    pieces: Vec<Piece>,
    pictures: Vec<u8>,
    /// The picture laid over all of it, `WASH_PER_CELL` pixels to a cell's side, if it has one.
    wash: Option<Image>,
}

/// How many pixels of a map's wash go along the side of a cell.
const WASH_PER_CELL: f32 = 8.0;

/// A square piece of a map's ground, as the original cuts it: what it covers, as (least x,
/// least z, most x, most z), and its faces, placed in the world already.
struct Piece {
    covers: (f32, f32, f32, f32),
    faces: Vec<LandFace>,
}

struct LandFace {
    points: [[f32; 3]; 3],
    normals: [[f32; 3]; 3],
    color: [f32; 3],
    picture: [[f32; 3]; 3],
}

/// A face ready to draw, given in its part's own room: painted with the picture `picture` if
/// it has one, each corner at `uv` on it; otherwise, and for a backend that does not paint
/// with pictures, its color is `color` plus the house color times `team`. Its back shows too
/// if its picture can be seen through.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Face {
    points: [[f32; 3]; 3],
    normals: [[f32; 3]; 3],
    color: [f32; 3],
    team: f32,
    picture: Option<usize>,
    uv: [[f32; 2]; 3],
    two_sided: bool,
    /// Its corners' vertices in its part, for a part that bends.
    corners: [usize; 3],
    /// 1 if the way its corners go round says its front, -1 if it says its back.
    winding: f32,
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
    /// The parts whose pictures slide over them, by name.
    slides: Vec<Slide>,
}

/// A part of a model: its faces, the part it sits in, where it is in that one, and whether it
/// is in the head.
#[derive(Clone, Debug, Default, PartialEq)]
struct Part {
    faces: Vec<Face>,
    parent: Option<usize>,
    transform: [[f32; 4]; 4],
    animation: Option<Animation>,
    bending: Option<Bending>,
    head: bool,
    /// How much of its picture slides over it each frame, along u and along v.
    slide: [f32; 2],
    /// Whether it is what lifts the barrel, which tilts up as far as the unit aims its tubes.
    lift: bool,
    /// Which of the model's weapon ends it is, for a part that marks one.
    muzzle: Option<usize>,
    /// For one of the model's lights, the faces that glow, each corner at `uv` on its picture.
    glows: Vec<Glow>,
}

/// A face of a light that adds its picture to what lies behind it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Glow {
    points: [[f32; 3]; 3],
    layer: usize,
    uv: [[f32; 2]; 3],
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
/// which it is of those given to the backend, and whether it has holes to see through; or a
/// light's glow, which is no surface; or one not on the disc.
enum Picture {
    Painted {
        image: Image,
        team: bool,
        layer: usize,
        holes: bool,
    },
    /// What lights add to what lies behind them, the backend's `layer`th picture.
    Glow {
        image: Image,
        layer: usize,
    },
    Missing,
}

impl Picture {
    /// Which of those given to the backend it is, if it is given.
    fn layer(&self) -> Option<usize> {
        match self {
            Picture::Painted { layer, .. } | Picture::Glow { layer, .. } => Some(*layer),
            Picture::Missing => None,
        }
    }
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

/// The pictures the original's models are painted with, once they are read, and then those of
/// the ground of the map of this number: squares `PICTURE_SIDE` wide, one after another, as
/// the backend takes them.
pub fn pictures(map: usize) -> Option<Vec<u8>> {
    let original = ORIGINAL.get()?;
    let land = land(map).map_or(&[][..], |land| &land.pictures);
    Some([&original.pictures[..], land].concat())
}

/// The wash laid over the ground of the map of this number, once its ground is read: its
/// pixels, rows of RGBA from the top, and how many there are across and down.
pub fn wash(map: usize) -> Option<(&'static [u8], usize, usize)> {
    let wash = land(map)?.wash.as_ref()?;
    Some((wash.pixels.as_flattened(), wash.width, wash.height))
}

/// How far along x and z the wash of the map of this number reaches over its ground; none
/// without one.
pub(super) fn wash_reach(map: usize) -> [f32; 2] {
    let reach = |pixels: usize| pixels as f32 / WASH_PER_CELL * CELL_SIDE;
    land(map)
        .and_then(|land| land.wash.as_ref())
        .map_or([0.0; 2], |wash| [reach(wash.width), reach(wash.height)])
}

/// The ground of the map of this number in the disc's list, read the first time it is needed
/// once the models are, since its pictures come after theirs.
fn land(map: usize) -> Option<&'static Land> {
    static LANDS: OnceLock<Vec<OnceLock<Option<Land>>>> = OnceLock::new();
    let original = ORIGINAL.get()?;
    let lands = LANDS.get_or_init(|| {
        (0..crate::maps::list().len())
            .map(|_| OnceLock::new())
            .collect()
    });
    lands
        .get(map)?
        .get_or_init(|| read_land(original, map))
        .as_ref()
}

fn read_land(original: &Original, map: usize) -> Option<Land> {
    let model = xbf::parse(&crate::maps::file(map, "test.xbf")?)?;
    let mut layers = original.layers;
    let mut images = Vec::new();
    let used: Vec<Picture> = model
        .textures
        .iter()
        .map(|texture| {
            let picture = read_picture(&original.archive, texture, layers);
            if let Picture::Painted { image, team, .. } = &picture {
                images.extend(square(image, *team));
                layers += 1;
            }
            picture
        })
        .collect();
    let mut pieces = Vec::new();
    for object in &model.objects {
        gather_land(object, &IDENTITY, &used, &mut pieces);
    }
    let cells = battlefield::terrain(map).cells();
    let wash = crate::maps::file(map, "test.CPT").and_then(|data| xbf::wash(&data, cells));
    Some(Land {
        pieces,
        pictures: images,
        wash,
    })
}

/// Adds the faces of a piece of a map's ground and what is in it, placed by `above`. The
/// original's rows run the other way along z from ours.
fn gather_land(
    object: &Object,
    above: &[[f32; 4]; 4],
    pictures: &[Picture],
    pieces: &mut Vec<Piece>,
) {
    let world = times(&object.transform, above);
    let flip = |[x, y, z]: [f32; 3]| [x, y, -z];
    let mut covers = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    let mut faces = Vec::new();
    for face in &object.faces {
        let picture = face.texture.and_then(|n| pictures.get(n));
        let color = paint(face, picture).map_or(UNPAINTED, |(color, _)| color);
        let corner = |n: usize| object.vertices[face.corners[n]];
        let points: [[f32; 3]; 3] =
            std::array::from_fn(|n| flip(point(&world, corner(n).position)));
        for [x, _, z] in points {
            covers = (
                covers.0.min(x),
                covers.1.min(z),
                covers.2.max(x),
                covers.3.max(z),
            );
        }
        faces.push(LandFace {
            points,
            normals: std::array::from_fn(|n| flip(normal(&world, corner(n).normal))),
            color,
            picture: match picture {
                Some(Picture::Painted { layer, .. }) => face.uv.map(|[u, v]| [u, v, *layer as f32]),
                _ => [NO_PICTURE; 3],
            },
        });
    }
    if !faces.is_empty() {
        pieces.push(Piece { covers, faces });
    }
    for child in &object.children {
        gather_land(child, &world, pictures, pieces);
    }
}

/// Draws the ground of the map of this number as the original shapes it, as much of it as
/// the view could show, if it can.
pub(super) fn draw_land(
    ground: &mut Vec<WorldVertex>,
    map: usize,
    fog: Option<&Fog>,
    view: &View,
    width: f32,
    height: f32,
) -> bool {
    /// How far past what the sand plane shows a piece still may, standing high.
    const ROOM: f32 = 512.0;
    let Some(land) = land(map) else {
        return false;
    };
    let seen = view.ground_seen(width, height);
    let shows = |&(x0, z0, x1, z1): &(f32, f32, f32, f32)| {
        seen.is_none_or(|(sx0, sz0, sx1, sz1)| {
            x1 >= sx0 - ROOM && x0 <= sx1 + ROOM && z1 >= sz0 - ROOM && z0 <= sz1 + ROOM
        })
    };
    for piece in land.pieces.iter().filter(|piece| shows(&piece.covers)) {
        for face in &piece.faces {
            for n in 0..3 {
                let [x, y, z] = face.points[n];
                let [r, g, b] = face.color;
                ground.push(WorldVertex {
                    x,
                    y,
                    z,
                    normal: face.normals[n],
                    r,
                    g,
                    b,
                    seen: battlefield::seen_at(fog, x, z),
                    level: y,
                    stain: [0.0; 3],
                    picture: face.picture[n],
                    team: [0.0; 3],
                });
            }
        }
    }
    true
}

/// Lays the spice on as much of the ground as the view could show, as the original does: on
/// each cell with spice a patch, and one more for each `SPICE_PER_PATCH` it has past that, each
/// a flat triangle of the spice picture over the ground under its middle, the patches of a
/// cell taken from the original's tables by where the cell is.
pub(super) fn draw_spice(
    decals: &mut Vec<WorldVertex>,
    terrain: &Terrain,
    fog: Option<&Fog>,
    view: &View,
    width: f32,
    height: f32,
) {
    /// How far past what the sand plane shows a patch still may.
    const ROOM: f32 = 2.0 * SPICE_REACH;
    let Some(spice) = ORIGINAL.get().and_then(|original| original.spice.as_ref()) else {
        return;
    };
    let (columns, rows) = terrain.cells();
    let (right, bottom) = terrain.extent();
    let (x0, z0, x1, z1) = view
        .ground_seen(width, height)
        .unwrap_or((0.0, 0.0, right, bottom));
    let cell = |at: f32, most: usize| ((at / CELL_SIDE).max(0.0) as usize).min(most);
    let (u0, v0) = (cell(x0 - ROOM, columns), cell(z0 - ROOM, rows));
    let (u1, v1) = (cell(x1 + ROOM, columns - 1), cell(z1 + ROOM, rows - 1));
    for v in v0..=v1 {
        for u in u0..=u1 {
            let (x, z) = battlefield::grid_to_world(u as f32 + 0.5, v as f32 + 0.5);
            spice.lay(decals, terrain, (u, v), battlefield::seen_at(fog, x, z));
        }
    }
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
    let mut given: Vec<(usize, Vec<u8>)> = pictures
        .values()
        .filter_map(|picture| match picture {
            Picture::Painted {
                image, team, layer, ..
            } => Some((*layer, square(image, *team))),
            Picture::Glow { image, layer } => Some((*layer, see_through(image))),
            Picture::Missing => None,
        })
        .collect();
    given.sort_by_key(|&(layer, _)| layer);
    let mut layers = given.len();
    let mut pictures: Vec<u8> = given.into_iter().flat_map(|(_, bytes)| bytes).collect();
    let spice = archive
        .read(&format!("{TEXTURES}{SPICE}"))
        .and_then(|data| xbf::tga(&data))
        .map(|image| {
            pictures.extend(see_through(&image));
            layers += 1;
            Spice::new(layers - 1)
        });
    (!shapes.is_empty()).then_some(Original {
        shapes,
        pictures,
        layers,
        archive,
        spice,
    })
}

/// Whether a picture's pixel is a hole: see-through, or the magenta that stands for it.
fn is_hole([r, g, b, a]: [u8; 4]) -> bool {
    a < 128 || (r > 229 && g < 26 && b > 229)
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
            let texel = if is_hole([r, g, b, a]) {
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

/// A picture made `PICTURE_SIDE` wide as `square` makes it, keeping how much of each pixel shows
/// in its alpha. Where nothing shows it takes the color of what does, so that the picture
/// does not darken at its edges as the backend shrinks it.
fn see_through(image: &Image) -> Vec<u8> {
    let shown: f32 = image.pixels.iter().map(|pixel| f32::from(pixel[3])).sum();
    let mean = std::array::from_fn::<u8, 3, _>(|n| {
        let sum: f32 = image
            .pixels
            .iter()
            .map(|pixel| f32::from(pixel[n]) * f32::from(pixel[3]))
            .sum();
        (sum / shown.max(1.0)) as u8
    });
    let mut bytes = Vec::with_capacity(PICTURE_SIDE * PICTURE_SIDE * 4);
    for y in 0..PICTURE_SIDE {
        for x in 0..PICTURE_SIDE {
            let [r, g, b, a] = image.pixels
                [y * image.height / PICTURE_SIDE * image.width + x * image.width / PICTURE_SIDE];
            let [r, g, b] = if a == 0 { mean } else { [r, g, b] };
            bytes.extend([r, g, b, a]);
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
                .filter(|picture| picture.layer().is_some())
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
        slides: model.slides,
        ..Shape::default()
    };
    for object in &model.objects {
        gather(object, None, &IDENTITY, false, &used, &mut shape);
    }
    // With no part to lift its barrel, it is what holds the end of its first weapon that does.
    if !shape.parts.iter().any(|part| part.lift)
        && let Some(holder) = shape
            .parts
            .iter()
            .find(|part| part.muzzle == Some(0))
            .and_then(|part| part.parent)
    {
        shape.parts[holder].lift = true;
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
    match archive
        .read(&format!("{TEXTURES}{name}"))
        .and_then(|data| xbf::tga(&data))
    {
        Some(image) if marks.contains('!') => Picture::Glow { image, layer },
        Some(image) => Picture::Painted {
            holes: image.pixels.iter().any(|&pixel| is_hole(pixel)),
            image,
            team: marks.contains('='),
            layer,
        },
        None => Picture::Missing,
    }
}

/// Whether an object is one of a model's lights, whose glowing faces show where the original
/// lights it up: a searchlight's beam and the spot it throws, a lamp, a flare.
fn is_light(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    ["spot", "light", "flare"]
        .iter()
        .any(|word| name.contains(word))
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
            let points: [[f32; 3]; 3] = std::array::from_fn(|n| corner(n).position);
            let normals: [[f32; 3]; 3] = std::array::from_fn(|n| corner(n).normal);
            let said = (0..3).map(|i| normals.iter().map(|n| n[i]).sum::<f32>());
            let front = winding_normal(&points)
                .into_iter()
                .zip(said)
                .map(|(a, b)| a * b)
                .sum::<f32>();
            let face = Face {
                points,
                normals,
                color,
                team,
                picture: match picture {
                    Some(Picture::Painted { layer, .. }) => Some(*layer),
                    _ => None,
                },
                uv: face.uv,
                two_sided: matches!(picture, Some(Picture::Painted { holes: true, .. })),
                corners: face.corners,
                winding: if front < 0.0 { -1.0 } else { 1.0 },
            };
            if head {
                for &position in &face.points {
                    shape.sunk = shape.sunk.max(-point(&world, position)[1]);
                }
            }
            faces.push(face);
        }
    }
    let glows = if is_light(&object.name) {
        object
            .faces
            .iter()
            .filter_map(
                |face| match face.texture.and_then(|n| pictures.get(n).copied()) {
                    Some(Picture::Glow { layer, .. }) => Some(Glow {
                        points: face.corners.map(|corner| object.vertices[corner].position),
                        layer: *layer,
                        uv: face.uv,
                    }),
                    _ => None,
                },
            )
            .collect()
    } else {
        Vec::new()
    };
    shape.parts.push(Part {
        faces,
        glows,
        parent,
        transform: object.transform,
        animation: object.animation.clone(),
        bending: object.bending.clone(),
        head,
        slide: shape
            .slides
            .iter()
            .find(|slide| slide.part == object.name)
            .map_or([0.0; 2], |slide| slide.step),
        lift: !shape.parts.iter().any(|part| part.lift) && object.name.contains("::1"),
        muzzle: object
            .name
            .strip_prefix(MUZZLE_MARK)
            .map(|rest| rest.trim_start_matches(|c: char| !c.is_ascii_digit()))
            .and_then(|rest| {
                let digits =
                    rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
                rest[..digits].parse().ok()
            }),
    });
    let this = shape.parts.len() - 1;
    for child in &object.children {
        gather(child, Some(this), &world, head, pictures, shape);
    }
}

/// The way a triangle's front faces by the way its corners go round, as long as it is big.
fn winding_normal([a, b, c]: &[[f32; 3]; 3]) -> [f32; 3] {
    let (u, v) = (
        [0, 1, 2].map(|i| b[i] - a[i]),
        [0, 1, 2].map(|i| c[i] - a[i]),
    );
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
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
        Some(Picture::Glow { .. }) => return None,
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

/// Whether the unit is drawn with one of the original's models.
pub(super) fn has_model(unit: &Unit) -> bool {
    shape_of(unit).is_some()
}

/// Which frame of its model's animation a unit is at, by what it is doing and the time of the
/// battle: its gun going up, a shot it has just fired, or one of its loops. A building goes
/// round its loop standing; a tower stays at the first.
fn frame(shape: &Shape, unit: &Unit, time: f32) -> usize {
    let find = |name: &str| {
        shape
            .sequences
            .iter()
            .find(|sequence| sequence.name == name)
            .map(|sequence| sequence.ranges.as_slice())
    };
    if unit.kind == UnitKind::Tower {
        return 0;
    }
    let played = (time * FRAMES_PER_SECOND) as usize;
    if unit.kind == UnitKind::Building {
        return find("Stationary").map_or(0, |ranges| {
            looped(ranges, played + unit.id as usize * FRAMES_APART)
        });
    }
    // Setting up what it deploys, or packing it up, as far as it has got.
    if unit.is_setting_up_or_packing() {
        let (name, share) = if unit.deployed {
            ("Deploy Gun", unit.popped)
        } else {
            ("Undeploy Gun", 1.0 - unit.popped)
        };
        if let Some(&[(first, last), ..]) = find(name) {
            return first + ((last - first) as f32 * share) as usize;
        }
    }
    if unit.role == Role::OrKobra
        && unit.popped > 0.0
        && let Some(&[(first, last), ..]) = find("Deploy Gun")
    {
        return first + ((last - first) as f32 * unit.popped.min(1.0)) as usize;
    }
    let fire = if unit.deployed {
        ["Deployed Fire", "Fire 1"]
    } else {
        ["Fire 0", "Fire 0"]
    };
    if unit.revealed > 0.0
        && let Some(frame) = fire.iter().find_map(|&name| find(name)).and_then(|ranges| {
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
        _ if unit.deployed => &["Deployed Idle 0", "Deploy Gun Hold"],
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

/// What a part of a model that marks one of its weapon ends starts with, before the number of
/// the end.
const MUZZLE_MARK: &str = ">>";

/// Where the unit stands as its model is drawn, its body and its head, which turns on its own
/// about its pivot.
fn places(shape: &Shape, unit: &Unit) -> (Place, Place) {
    let pivot = shape.pivot.unwrap_or_default();
    match unit.kind {
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
    }
}

/// Where each part of the model is within it at a frame, the barrel lifted as far as the unit
/// aims its tubes.
fn pose(shape: &Shape, unit: &Unit, frame: usize) -> Vec<[[f32; 4]; 4]> {
    let (sin, cos) = unit.tube_pitch.sin_cos();
    // Turned about its own across, so that its front, along -z, comes up.
    let lifted = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, cos, sin, 0.0],
        [0.0, -sin, cos, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut worlds: Vec<[[f32; 4]; 4]> = Vec::with_capacity(shape.parts.len());
    for part in &shape.parts {
        let above = part.parent.map_or(&IDENTITY, |parent| &worlds[parent]);
        let at = part.at(frame);
        let at = if part.lift { times(&lifted, &at) } else { at };
        worlds.push(times(&at, above));
    }
    worlds
}

/// Where the ends of the weapons of the unit's model are on the battlefield, by their numbers,
/// if it has a model with any.
pub fn muzzles(unit: &Unit) -> Vec<(f32, f32, f32)> {
    let Some(shape) = shape_of(unit) else {
        return Vec::new();
    };
    let (body, head) = places(shape, unit);
    let pivot = shape.pivot.unwrap_or_default();
    let worlds = pose(shape, unit, frame(shape, unit, 0.0));
    let mut ends: Vec<(usize, (f32, f32, f32))> = shape
        .parts
        .iter()
        .zip(&worlds)
        .filter_map(|(part, world)| {
            let place = to_place([world[3][0], world[3][1], world[3][2]]);
            let [x, y, z] = if part.head {
                head.point(std::array::from_fn(|i| place[i] - pivot[i]))
            } else {
                body.point(place)
            };
            Some((part.muzzle?, (x, y, z)))
        })
        .collect();
    ends.sort_by_key(|&(n, _)| n);
    ends.into_iter().map(|(_, end)| end).collect()
}

/// Draws the original's model of the unit in its house color, as it is at this time of the
/// battle, if there is one to draw.
pub(super) fn draw(canvas: &mut Canvas, unit: &Unit, team: [f32; 3], time: f32) -> bool {
    let Some(shape) = shape_of(unit) else {
        return false;
    };
    let pivot = shape.pivot.unwrap_or_default();
    let (body, head) = places(shape, unit);
    let frame = frame(shape, unit, time);
    let worlds = pose(shape, unit, frame);
    // Pictures slide all the time, but a machine's tracks only as it goes.
    let slid = if unit.kind == UnitKind::Vehicle && !unit.is_moving() {
        0.0
    } else {
        time * FRAMES_PER_SECOND
    };
    let mut glows = Vec::new();
    let mut model = Model::new(canvas, body);
    for (part, world) in shape.parts.iter().zip(&worlds) {
        let (mut model, from) = if part.head {
            (model.at(head), pivot)
        } else {
            (model.at(body), [0.0; 3])
        };
        // Only how far into its picture it has slid: a whole picture on looks the same.
        let slide = part.slide.map(|step| (step * slid).fract());
        let place = if part.head { head } else { body };
        for glow in &part.glows {
            glows.extend((0..3).map(|n| {
                let at = to_place(point(world, glow.points[n]));
                let [x, y, z] = place.point(std::array::from_fn(|i| at[i] - from[i]));
                let [u, v] = glow.uv[n];
                WorldVertex {
                    picture: [u + slide[0], v + slide[1], glow.layer as f32],
                    level: GLOW,
                    ..WorldVertex::shiny((x, y, z), [0.0, 1.0, 0.0], [1.0; 3], 0.0)
                }
            }));
        }
        for face in &part.faces {
            let color = std::array::from_fn(|i| face.color[i] + face.team * team[i]);
            // A part that bends has its corners where the frame has them, its face as flat.
            let (local, normals) = match &part.bending {
                Some(bending) => {
                    let mut points = face.points;
                    for (point, &corner) in points.iter_mut().zip(&face.corners) {
                        *point = bending.at(frame, corner).unwrap_or(*point);
                    }
                    let way = winding_normal(&points).map(|c| c * face.winding);
                    (points, [way; 3])
                }
                None => (face.points, face.normals),
            };
            let points = local.map(|p| {
                let place = to_place(point(world, p));
                std::array::from_fn(|i| place[i] - from[i])
            });
            let normals = normals.map(|n| to_place(normal(world, n)));
            let picture = face.picture.map_or([NO_PICTURE; 3], |layer| {
                face.uv
                    .map(|[u, v]| [u + slide[0], v + slide[1], layer as f32])
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
                face.two_sided,
            );
        }
    }
    canvas.decals.extend(glows);
    true
}

/// What a decal's level is for a light's glow, which adds all of its picture.
const GLOW: f32 = -1.0;

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
            holes: true,
        };
        let near = |a: f32, b: f32| (a - b).abs() < 1e-5;
        let (color, team) = paint(&face(0.25), Some(&house)).unwrap();
        assert!(color == [0.0; 3] && near(team, 200.0 / 255.0));
        let (color, team) = paint(&face(0.75), Some(&house)).unwrap();
        assert!(near(color[0], 100.0 / 255.0) && near(color[1], 50.0 / 255.0) && team == 0.0);
        let glow = Picture::Glow {
            image: Image {
                width: 0,
                height: 0,
                pixels: Vec::new(),
            },
            layer: 0,
        };
        assert_eq!(paint(&face(0.25), Some(&glow)), None);
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
        assert_eq!(
            frame(&shape, &unit, 0.2),
            105,
            "a building goes round its loop"
        );
        unit.kind = UnitKind::Tower;
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
    fn what_can_be_seen_through_shows_its_back_turned_to_the_camera() {
        use super::super::models::Watchers;
        // Facing down, away from the sun, with the camera above it.
        let points = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let down = [[0.0, -1.0, 0.0]; 3];
        let drawn = |two_sided: bool| {
            let mut canvas = Canvas {
                watchers: Some(Watchers {
                    eye: [0.0, 100.0, 0.0],
                    sun: battlefield::SUN,
                    camera: true,
                    mirrored: false,
                }),
                ..Canvas::default()
            };
            let place = Place::turned([0.0; 3], 0.0);
            let paint = Paint {
                color: [0.5; 3],
                shine: 0.0,
            };
            let mut model = Model::new(&mut canvas, place);
            model.pictured(points, down, paint, [NO_PICTURE; 3], [0.0; 3], two_sided);
            canvas.objects
        };
        assert!(drawn(false).is_empty(), "its back is left out");
        let back = drawn(true);
        assert_eq!(back.len(), 3);
        assert!(back.iter().all(|corner| corner.normal[1] > 0.99));
    }

    #[test]
    fn a_part_inside_a_marker_still_shows_but_the_marker_does_not() {
        assert!(is_marker("#AT_trike_parent"));
        assert!(is_marker("SLCTattrike"));
        assert!(is_marker("?Spotlight"));
        assert!(!is_marker("::0rot"));
        assert!(!is_marker("~~0Body"));
    }

    #[test]
    fn a_cell_full_of_spice_is_laid_with_four_patches_about_it_and_one_without_none() {
        let terrain = Terrain::open_rock().with_spice((100, 100), (1, 1));
        let spice = Spice::new(3);
        let mut decals = Vec::new();
        spice.lay(&mut decals, &terrain, (101, 100), 1.0);
        assert!(decals.is_empty());
        spice.lay(&mut decals, &terrain, (100, 100), 1.0);
        assert_eq!(decals.len(), 4 * 3);
        let (x, z) = battlefield::grid_to_world(100.5, 100.5);
        let reach = SPICE_SHIFT * 2.0_f32.sqrt() + SPICE_REACH + SPICE_REACH_JITTER;
        for corner in &decals {
            assert!((corner.x - x).hypot(corner.z - z) <= reach);
            assert_eq!(corner.y, SPICE_LIFT);
            assert_eq!(corner.picture[2], 3.0);
        }
    }
}
