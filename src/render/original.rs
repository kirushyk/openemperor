//! The original game's own models, read off its disc when it is mounted, drawn in place of
//! ours for the machines, buildings and towers it has them for. They are measured in the
//! world's own coords; each looks to -z, with its up y and its right x. Each face takes the
//! colors its picture has under it, and the house color where the picture leaves room for it.

use super::Canvas;
use super::frontend;
use super::lettering::{self, Lettering};
use super::models::{Model, Paint, Place};
use crate::archive::{self, Archive};
use crate::battlefield::{self, CELL_SIDE, Terrain, View};
use crate::fog::Fog;
use crate::game::{CursorLook, Faction};
use crate::platform::{NO_PICTURE, Vertex, WorldVertex};
use crate::units::{
    CrateKind, Flight, Frigate, Gender, Harvest, Prop, Role, Unit, UnitKind, WallPiece, Wound,
};
use crate::weapons::{BODY_SECONDS, BODY_SINK_SECONDS, Body, Made, Noise, REVEAL_SECONDS, Tornado};
use crate::xbf::{self, Animation, Bending, Image, Object, Sequence, Slide};
pub(super) use crate::xbf::{point, times};
use std::cmp::Ordering;
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
/// `spice.ini` has it.
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
        (
            Role::Windtrap,
            Some(Atreides),
            "Buildings\\AT_Windtrap_H0.XbF",
        ),
        (
            Role::Windtrap,
            Some(Harkonnen),
            "Buildings\\HK_Windtrap_H0.XBF",
        ),
        (Role::Windtrap, Some(Ordos), "Buildings\\or_windtrap_H0.XbF"),
        (
            Role::Prop(Prop::Barrel),
            None,
            "Buildings\\IN_barrel_h0.xbf",
        ),
        (Role::Prop(Prop::Drum), None, "Buildings\\IN_barrel1_h0.xbf"),
        (
            Role::Prop(Prop::FactoryFrigate),
            Some(Atreides),
            "Buildings\\AT_FactoryFrigate_H0.xbf",
        ),
        (
            Role::Prop(Prop::FactoryFrigate),
            Some(Ordos),
            "Buildings\\Or_FactoryFrigate_H0.xbf",
        ),
        (
            Role::Prop(Prop::FactoryFrigate),
            None,
            "Buildings\\HK_FactoryFrigate_H0.xbf",
        ),
        (
            Role::Prop(Prop::Spotlight),
            None,
            "Buildings\\GP_IN_Spotlight_H0.xbf",
        ),
        (
            Role::Prop(Prop::GreenSmoke),
            None,
            "Buildings\\GP_SFX_GreenSmoke_H0.xbf",
        ),
        (
            Role::Prop(Prop::CrawlerWreck),
            None,
            "Buildings\\IN_SandCrawlerWreck_h0.xbf",
        ),
        (
            Role::Prop(Prop::HarvesterWreck),
            None,
            "Buildings\\IN_HarvesterWreck_h0.xbf",
        ),
        (
            Role::Prop(Prop::BuggyWreck),
            None,
            "Buildings\\IN_BuggyWreck_h0.xbf",
        ),
        (
            Role::Prop(Prop::HungFigure),
            None,
            "Buildings\\AK_IN_hungfigure2_H0.xbf",
        ),
        (
            Role::Starport,
            Some(Atreides),
            "Buildings\\AT_StarPort_H0.XbF",
        ),
        (
            Role::Starport,
            Some(Harkonnen),
            "Buildings\\hk_starport_H0.XBF",
        ),
        (Role::Starport, Some(Ordos), "Buildings\\Or_StarPort_H0.XbF"),
        (Role::Palace(Atreides), None, "Buildings\\AT_Palace_H0.XbF"),
        (Role::Palace(Harkonnen), None, "Buildings\\hk_palace_h0.XBF"),
        (Role::Palace(Ordos), None, "Buildings\\OR_Palace_H0.XBF"),
        (Role::TlFleshVat, None, "Buildings\\TL_Fleshvat_H0.xbf"),
        (Role::IxResCentre, None, "Buildings\\IX_ResCentre_H0.xbf"),
        (Role::GuPalace, None, "Buildings\\GU_Palace_H0.xbf"),
        (Role::ImBarracks, None, "Buildings\\IM_Barracks_H0.xbf"),
        (Role::FrCamp, None, "Buildings\\FR_Camp_H0.xbf"),
        (Role::FrFremen, None, "Units\\FR_Fremen_H0.xbf"),
        (Role::FrAdvFremen, None, "Units\\FR_ADVFremen_H0.xbf"),
        (Role::HkGeneral, None, "Units\\HK_General_H0.XBF"),
        (
            Role::Civilian(Gender::Male),
            None,
            "Units\\IN_MaleCiv_H0.xbf",
        ),
        (
            Role::Civilian(Gender::Female),
            None,
            "Units\\IN_FemaleCiv_H0.xbf",
        ),
        (Role::ImSardaukar, None, "Units\\IM_Sardaukar_H0.xbf"),
        (Role::TlContaminator, None, "Units\\TL_Contaminator_H0.xbf"),
        (Role::IxInfiltrator, None, "Units\\IX_Infiltrator_H0.xbf"),
        (Role::GuMaker, None, "Units\\GU_Maker_H0.xbf"),
    ]
};

/// The original's model of a tornado.
const TORNADO: &str = "Units\\IN_Tornado_H0.xbf";
/// The original's model of the starport's frigate, the rules' `[Frigate]`.
const FRIGATE: &str = "Units\\G_Frigate_H0.xbf";

/// The picture on the build button of each role, for every house or for one, and the grey one
/// it shows while it cannot be made, as the original's `ArtIni.txt` names them. Ordos have no
/// helipad on the disc.
const ICONS: &[(Role, Option<Faction>, &str, &str)] = {
    use Faction::{Atreides, Harkonnen, Ordos};
    &[
        (Role::Scout, Some(Atreides), "AT_Scout", "grey_AT_Scout"),
        (Role::Scout, Some(Harkonnen), "HK_Scout", "grey_HK_Scout"),
        (Role::Scout, Some(Ordos), "OR_Scout", "grey_OR_Scout"),
        (
            Role::Engineer,
            Some(Atreides),
            "AT_Engineer",
            "Grey_AT_Engineer",
        ),
        (
            Role::Engineer,
            Some(Harkonnen),
            "HK_Engineer",
            "Grey_HK_Engineer",
        ),
        (
            Role::Engineer,
            Some(Ordos),
            "OR_Engineer",
            "Grey_OR_Engineer",
        ),
        (Role::AtInfantry, None, "AT_Inf", "grey_AT_Inf"),
        (Role::AtSniper, None, "AT_Sniper", "grey_AT_Sniper"),
        (Role::AtKindjal, None, "AT_DeployInf", "grey_AT_DeployInf"),
        (Role::HkLightInf, None, "HK_LightInf", "grey_HK_LightInf"),
        (Role::HkTrooper, None, "HK_Trooper", "grey_HK_Trooper"),
        (Role::HkFlamer, None, "HK_Flamer", "grey_HK_Flamer"),
        (Role::OrChemical, None, "OR_Chemical", "Grey_OR_Chemical"),
        (Role::OrSaboteur, None, "OR_Saboteur", "Grey_OR_Saboteur"),
        (Role::OrMortar, None, "OR_mortar", "grey_OR_mortar"),
        (Role::AtTrike, None, "At_Trike", "grey_AT_Trike"),
        (Role::AtRepairUnit, None, "AT_Repair", "grey_AT_Repair"),
        (Role::AtMongoose, None, "At_Mongoose", "grey_AT_Mongoose"),
        (
            Role::AtMinotaurus,
            None,
            "At_Minotaurus",
            "grey_AT_Minotaurus",
        ),
        (Role::AtApc, None, "AT_APC", "grey_AT_APC"),
        (Role::AtOrni, None, "AT_Ornithopter", "grey_AT_Ornithopter"),
        (Role::AtAdp, None, "AT_Drone", "grey_AT_Drone"),
        (Role::HkAssault, None, "HK_AsTank", "grey_HK_AsTank"),
        (Role::HkInkVine, None, "HK_InkVine", "grey_HK_InkVine"),
        (
            Role::HkMissile,
            None,
            "HK_MissileTank",
            "grey_HK_MissileTank",
        ),
        (Role::HkBuzzsaw, None, "HK_BuzzSaw", "grey_HK_BuzzSaw"),
        (Role::HkFlame, None, "HK_FlameTank", "grey_HK_FlameTank"),
        (
            Role::HkDevastator,
            None,
            "HK_Devestator",
            "grey_HK_Devestator",
        ),
        (Role::HkGunship, None, "HK_Gunship", "grey_HK_Gunship"),
        (Role::HkAdp, None, "HK_Airgun", "grey_HK_Airgun"),
        (Role::OrDustScout, None, "OR_DustScout", "grey_OR_DustScout"),
        (Role::OrLaserTank, None, "OR_LaserTank", "grey_OR_LaserTank"),
        (Role::OrApc, None, "OR_APC", "grey_OR_APC"),
        (Role::OrKobra, None, "OR_Kobra", "grey_OR_Kobra"),
        (Role::OrEits, None, "OR_EITS", "grey_OR_EITS"),
        (Role::OrAdp, None, "OR_ADD", "grey_OR_ADD"),
        (
            Role::AdvCarryall,
            Some(Atreides),
            "AT_ADVCarryall",
            "grey_AT_ADVCarryall",
        ),
        (
            Role::AdvCarryall,
            Some(Harkonnen),
            "HK_ADVCarryall",
            "grey_HK_ADVCarryall",
        ),
        (
            Role::AdvCarryall,
            Some(Ordos),
            "OR_ADVCarryall",
            "grey_OR_ADVCarryall",
        ),
        (Role::Carryall, None, "G_Carryall", "grey_G_Carryall"),
        (Role::Mcv, None, "G_MCV", "grey_G_MCV"),
        (Role::Harvester, None, "G_Harvester", "grey_G_Harvester"),
        (
            Role::Barracks,
            Some(Atreides),
            "AT_Barracks",
            "grey_AT_Barracks",
        ),
        (
            Role::Barracks,
            Some(Harkonnen),
            "HK_Barraks",
            "grey_HK_Barraks",
        ),
        (
            Role::Barracks,
            Some(Ordos),
            "OR_Barracks",
            "grey_OR_Barracks",
        ),
        (
            Role::Factory,
            Some(Atreides),
            "AT_Factory",
            "grey_AT_Factory",
        ),
        (
            Role::Factory,
            Some(Harkonnen),
            "HK_Factory",
            "grey_HK_Factory",
        ),
        (Role::Factory, Some(Ordos), "OR_Factory", "grey_OR_Factory"),
        (Role::Hanger, Some(Atreides), "AT_Hanger", "grey_AT_Hanger"),
        (
            Role::Hanger,
            Some(Harkonnen),
            "HK_Hang_ico",
            "grey_HK_Hang_ico",
        ),
        (Role::Hanger, Some(Ordos), "OR_Hanger", "grey_OR_Hanger"),
        (
            Role::Helipad,
            Some(Atreides),
            "AT_Helipad",
            "grey_AT_Helipad",
        ),
        (
            Role::Helipad,
            Some(Harkonnen),
            "HK_Helipad",
            "grey_HK_Helipad",
        ),
        (
            Role::Outpost,
            Some(Atreides),
            "AT_Outpost",
            "grey_AT_Outpost",
        ),
        (
            Role::Outpost,
            Some(Harkonnen),
            "HK_Outpost",
            "grey_HK_Outpost",
        ),
        (Role::Outpost, Some(Ordos), "OR_Outpost", "grey_OR_Outpost"),
        (
            Role::Refinery,
            Some(Atreides),
            "AT_Refinery",
            "grey_AT_Refinery",
        ),
        (
            Role::Refinery,
            Some(Harkonnen),
            "HK_Refinery",
            "grey_HK_Refinery",
        ),
        (
            Role::Refinery,
            Some(Ordos),
            "OR_Refinery",
            "grey_OR_Refinery",
        ),
        (Role::Palace(Atreides), None, "AT_Palace", "grey_AT_Palace"),
        (Role::Palace(Harkonnen), None, "HK_Palace", "grey_HK_Palace"),
        (Role::Palace(Ordos), None, "OR_Palace", "grey_OR_Palace"),
        (Role::AtPillbox, None, "AT_Pillbox", "grey_AT_Pillbox"),
        (
            Role::AtRocketTurret,
            None,
            "AT_RocketTurret",
            "grey_AT_RocketTurret",
        ),
        (
            Role::HkFlameTurret,
            None,
            "HK_FlameTurret",
            "grey_HK_FlameTurret",
        ),
        (Role::HkGunTurret, None, "HK_GunTurret", "grey_HK_GunTurret"),
        (Role::OrGasTurret, None, "OR_gasturret", "grey_OR_gasturret"),
        (
            Role::OrPopUpTurret,
            None,
            "OR_popupturret",
            "grey_OR_popupturret",
        ),
        (
            Role::Windtrap,
            Some(Atreides),
            "AT_Windtrap",
            "grey_AT_Windtrap",
        ),
        (
            Role::Windtrap,
            Some(Harkonnen),
            "HK_Windtrap",
            "grey_HK_Windtrap",
        ),
        (
            Role::Windtrap,
            Some(Ordos),
            "OR_Windtrap",
            "grey_OR_Windtrap",
        ),
        (
            Role::Starport,
            Some(Atreides),
            "AT_StarPort",
            "grey_AT_StarPort",
        ),
        (
            Role::Starport,
            Some(Harkonnen),
            "HK_StarPort",
            "grey_HK_StarPort",
        ),
        (
            Role::Starport,
            Some(Ordos),
            "OR_StarPort",
            "grey_OR_StarPort",
        ),
        // Every house's, as the original's ArtIni.txt has it.
        (Role::Wall, None, "HK_Wall", "grey_HK_Wall"),
        (Role::TlFleshVat, None, "TL_FleshVat", "grey_TL_FleshVat"),
        (Role::IxResCentre, None, "IX_ResCentre", "grey_IX_ResCentre"),
        (Role::GuPalace, None, "GU_Palace", "Grey_Gu_Palace"),
        (Role::ImBarracks, None, "IM_Barracks", "Grey_IM_Barracks"),
        (Role::FrCamp, None, "FR_Camp", "Grey_FR_Camp"),
        (
            Role::TlContaminator,
            None,
            "TL_Contaminator",
            "grey_TL_Contaminator",
        ),
        (
            Role::IxInfiltrator,
            None,
            "IX_Infiltrator",
            "grey_IX_Infiltrator",
        ),
        (Role::GuMaker, None, "GU_Maker", "grey_GU_Maker"),
        (Role::ImSardaukar, None, "IM_Sardaukar", "grey_IM_Sardaukar"),
        (Role::FrFremen, None, "FR_Fremen", "Grey_FR_Fremen"),
    ]
};

/// The build buttons of what only the Upgrades menu shows, for each house: the pad a refinery
/// gains, the original's RefineryDock, and the construction yard, lit and grey.
const UPGRADE_ICONS: [(Faction, [(&str, &str); 2]); 3] = [
    (
        Faction::Atreides,
        [
            ("AT_RefineryPAD", "grey_AT_RefineryPAD"),
            ("Ico2_acy", "grey_Ico2_acy"),
        ],
    ),
    (
        Faction::Harkonnen,
        [
            ("HK_RefineryPAD", "grey_HK_RefineryPAD"),
            ("Ico2_hcy", "grey_Ico2_hcy"),
        ],
    ),
    (
        Faction::Ordos,
        [
            ("OR_RefineryPAD", "grey_OR_RefineryPAD"),
            ("ORico2_acy", "grey_ORico2_acy"),
        ],
    ),
];

/// What the name of the turret a soldier fires from once dug in has in it.
const DUG_IN_TURRET: &str = "::1";
/// How far a gun a soldier sets up moves about at most, in its model's units: it only recoils.
const SET_UP_GUN_STILL: f32 = 8.0;
/// What the name of a searchlight's beam has in it.
const SEARCHLIGHT: &str = "spotlight";

/// The part of a Kobra's model that swings its raised cannon round.
const SWIVEL: &str = "gunpivot";

/// Each house's pieces of wall, in the order of `WallPiece`: named as the disc's models of
/// them are, its `Xaf` and then "", "_End", "_Middle", "_Corner", "_TJoin" or "_Cross" before
/// the detail and damage letters.
const WALLS: [(Faction, [&str; 6]); 3] = [
    (
        Faction::Atreides,
        [
            "Buildings\\AT_Wall_H0.xbf",
            "Buildings\\AT_Wall_End_H0.xbf",
            "Buildings\\AT_Wall_Middle_H0.xbf",
            "Buildings\\AT_Wall_Corner_H0.xbf",
            "Buildings\\AT_Wall_TJoin_H0.xbf",
            "Buildings\\AT_Wall_Cross_H0.xbf",
        ],
    ),
    (
        Faction::Harkonnen,
        [
            "Buildings\\HK_Wall_H0.xbf",
            "Buildings\\HK_Wall_End_H0.xbf",
            "Buildings\\HK_Wall_Middle_H0.xbf",
            "Buildings\\HK_Wall_Corner_H0.xbf",
            "Buildings\\HK_Wall_TJoin_H0.xbf",
            "Buildings\\HK_Wall_Cross_H0.xbf",
        ],
    ),
    (
        Faction::Ordos,
        [
            "Buildings\\OR_Wall_H0.xbf",
            "Buildings\\OR_Wall_End_H0.xbf",
            "Buildings\\OR_Wall_Middle_H0.xbf",
            "Buildings\\OR_Wall_Corner_H0.xbf",
            "Buildings\\OR_Wall_TJoin_H0.xbf",
            "Buildings\\OR_Wall_Cross_H0.xbf",
        ],
    ),
];

/// The original's model of each kind of scenery, by the rules' name for it.
#[rustfmt::skip]
/// The disc's models of the rules' extra kinds, by their rules' names.
const EXTRAS: &[(&str, &str)] = &[
    ("INYak", "Units\\in_yak_h0.xbf"),
    ("INYakHauder", "Units\\in_yakhauder_h0.xbf"),
    ("INYakRider", "Units\\in_yakrider_h0.xbf"),
    ("ATGeneral", "Units\\at_general_h0.xbf"),
    ("ATMilitia", "Units\\at_militia_h0.xbf"),
    ("ATSonicTank", "Units\\at_sonictank_h0.xbf"),
    ("ATINBrancastle", "Buildings\\at_in_brancastle_h0.xbf"),
    ("BeaconFlare", "Buildings\\beaconflare_h0.xbf"),
    ("FakeHarvester", "Units\\g_harvester_h0.xbf"),
    ("GUNIABTank", "Units\\gu_niabtank_h0.xbf"),
    ("GURefinery", "Buildings\\gu_refinery_h0.xbf"),
    ("GUWormhead", "Buildings\\gu_wormhead_h0.xbf"),
    ("GUbubble", "Buildings\\gu_bubble_h0.xbf"),
    ("Guglasstank", "Buildings\\gu_glasstank_h0.xbf"),
    ("GUMegaCannon", "Buildings\\in_gu_megacannon_h0.xbf"),
    ("GUNIAP", "Units\\gu_in_niap_h0.xbf"),
    ("GUpyramid", "Buildings\\gu_pyramid_h0.xbf"),
    ("IMADVSardaukar", "Units\\im_advsardaukar_h0.xbf"),
    ("IMGeneral", "Units\\im_general_h0.xbf"),
    ("IMMonument", "Buildings\\im_monument_h0.xbf"),
    ("INBuggy", "Units\\in_buggy_h0.xbf"),
    ("INFRBiker", "Units\\in_fr_biker_h0.xbf"),
    ("INGUJackelHouse", "Buildings\\in_gu_jackelhouse_h0.xbf"),
    ("INIMShed", "Buildings\\in_im_shed_h0.xbf"),
    ("INIMShed2", "Buildings\\in_im_shed2_h0.xbf"),
    ("INIXDome", "Buildings\\in_ix_dome_h0.xbf"),
    ("INIXTower", "Buildings\\in_ix_tower_h0.xbf"),
    ("INIXTower2", "Buildings\\in_ix_tower2_h0.xbf"),
    ("INIXTransport", "Units\\in_ix_transport_h0.xbf"),
    ("INMedicalVehicle", "Units\\in_medical_h0.xbf"),
    ("INSandCrawler", "Units\\in_sandcrawler_h0.xbf"),
    ("INTLTransport", "Units\\in_tl_transport_h0.xbf"),
    ("INTLWalker", "Units\\in_tl_walker_h0.xbf"),
    ("INTLWindtrap", "Buildings\\in_tl_windtrap_h0.xbf"),
    ("IXMegaCannon", "Buildings\\in_ix_megacannon_h0.xbf"),
    ("IXWall", "Buildings\\ix_wall_h0.xbf"),
    ("IXProjector", "Units\\ix_radar_h0.xbf"),
    ("IXRefinery", "Buildings\\ix_refinery_h0.xbf"),
    ("IXScientist", "Units\\ix_scientist_h0.xbf"),
    ("IXSlave", "Units\\ix_slave_h0.xbf"),
    ("IXWindtrap", "Buildings\\ix_windtrap_h0.xbf"),
    ("ORAATrooper", "Units\\or_aatrooper_h0.xbf"),
    ("ORDeviator", "Units\\or_deviator_h0.xbf"),
    ("SMQuad", "Units\\sm_quad_h0.xbf"),
    ("StuntATADVCarryall", "Units\\at_carryall_h0.xbf"),
    ("TLINGreenhouse", "Buildings\\tl_in_greenhouse_h0.xbf"),
    ("TLLeech", "Units\\tl_leech_h0.xbf"),
    ("TLScientist", "Units\\tl_scientist_h0.xbf"),
    ("TLTurret", "Buildings\\tl_turret_h0.xbf"),
    ("Whale", "Buildings\\cn_in_whale_h0.xbf"),
];

const SCENERY: &[(&str, &str)] = &[
    ("AKINDrKynes", "Buildings\\AK_IN_DrKynes_H0.xbf"),
    ("AKINNoddingDonkey", "Buildings\\AK_IN_NoddingDonkey_H0.xbf"),
    ("AKINhungfigure", "Buildings\\AK_IN_hungfigure_H0.xbf"),
    ("AKINhungfigure3", "Buildings\\AK_IN_hungfigure3_H0.xbf"),
    ("AKINhungfigure4", "Buildings\\AK_IN_hungfigure4_H0.xbf"),
    ("ATINHouse2", "Buildings\\AT_IN_house2_H0.xbf"),
    ("ATINHouse3", "Buildings\\AT_IN_house3_H0.xbf"),
    ("ATINHouse4", "Buildings\\AT_IN_house4_H0.xbf"),
    ("ATINStPauls", "Buildings\\AT_IN_Stpauls_H0.xbf"),
    ("ATINSultan", "Buildings\\AT_IN_Sultan_H0.xbf"),
    ("BirdBuilding", "Buildings\\AK_IN_Vultures_H0.xbf"),
    ("CNINATTree", "Buildings\\CN_IN_ATTree_H0.Xbf"),
    ("CNINATTree2", "Buildings\\CN_IN_ATTree2_H0.xbf"),
    ("CNINATTree3", "Buildings\\CN_IN_ATTree3_H0.xbf"),
    ("CNINATTree4", "Buildings\\CN_IN_ATTree4_H0.xbf"),
    (
        "GPSFXGreensmokeNL",
        "Buildings\\GP_SFX_GreenSmoke_NL_H0.xbf",
    ),
    ("GPSFXLavasmoke", "Buildings\\GP_SFX_LavaSmoke_H0.xbf"),
    ("GPSFXLavasmokeNL", "Buildings\\GP_SFX_LavaSmoke_NL_H0.xbf"),
    ("HKINApartments", "Buildings\\HK_IN_Apartments_H0.xbf"),
    ("HKINGiediRef", "Buildings\\HK_IN_house_H0.xbf"),
    ("HKINHouse", "Buildings\\HK_IN_house_H0.xbf"),
    ("HKINHouse2", "Buildings\\HK_IN_house2_H0.xbf"),
    ("HKINHouse3", "Buildings\\HK_IN_house3_H0.xbf"),
    ("HKINHouse5", "Buildings\\HK_IN_house5_H0.xbf"),
    ("HKINHut", "Buildings\\HK_IN_Hut_H0.XBF"),
    ("HKINMossPalace", "Buildings\\HK_IN_Mosspalace_H0.xbf"),
    ("HKINSultan", "Buildings\\HK_IN_Sultan_H0.xbf"),
    ("HKINTYower", "Buildings\\HK_IN_TYower_H0.xbf"),
    ("HKINTennament", "Buildings\\HK_IN_Tennament_H0.xbf"),
    ("HKINTennament2", "Buildings\\HK_IN_Tennament_H0.xbf"),
    ("HKINTownhall", "Buildings\\HK_IN_Townhall_H0.xbf"),
    ("HKINTrafford", "Buildings\\HK_IN_Trafford_H0.xbf"),
    ("HKINTwafford", "Buildings\\HK_IN_Twafford_H0.xbf"),
    ("HLINElectricVent", "Buildings\\HL_IN_ElectricVent_H0.xbf"),
    ("HLINLighGate", "Buildings\\HL_IN_LightningGate_H0.xbf"),
    ("HLINMHut2", "Buildings\\HL_IN_Mhut2_H0.xbf"),
    ("HLINMhut", "Buildings\\HL_IN_MHut_H0.xbf"),
    ("HLINOxygen", "Buildings\\HL_IN_Oxygen_H0.xbf"),
    ("INCrash", "Buildings\\IN_aircrash_h0.xbf"),
    ("INFRCampFire", "Buildings\\IN_FR_Campfire_H0.xbf"),
    ("INFRTent", "Buildings\\IN_FR_Tent_H0.xbf"),
    ("INFRTent2", "Buildings\\IN_FR_Tent2_H0.xbf"),
    ("INHouse", "Buildings\\IN_House_H0.XBF"),
    ("INMartHouse", "Buildings\\IN_MartHouse_H0.XBF"),
    ("INMartHouse2", "Buildings\\IN_MartHouse2_H0.XBF"),
    ("INMedicalWreck", "Buildings\\IN_MedicalWreck_h0.xbf"),
    ("INPalace", "Buildings\\IN_palace_H0.xbf"),
    ("INStore", "Buildings\\IN_Store_H0.XBF"),
    ("INStore2", "Buildings\\IN_Store2_H0.XBF"),
    ("INWorkshop", "Buildings\\IN_Workshop_H0.XBF"),
    ("IXTransport", "Buildings\\IN_IX_Transportwreck_H0.xbf"),
    ("ORINBubbleHouse", "Buildings\\or_in_bubble_h0.XBF"),
    ("ORINIndi", "Buildings\\OR_IN_Indi_H0.xbf"),
    ("ORINTower", "Buildings\\OR_IN_Tower_H0.xbf"),
    ("ORINTower2", "Buildings\\OR_IN_Tower2_H0.xbf"),
    ("ORINTownhall", "Buildings\\OR_IN_TownHall_H0.xbf"),
    ("ORINhall", "Buildings\\Or_IN_Hall_H0.xbf"),
    ("SMStarport", "Buildings\\SM_StarPort_H0.xbf"),
    ("Seaguls", "Buildings\\CN_IN_Seaguls_H0.xbf"),
    ("TLTransport", "Buildings\\IN_TL_Transportwreck_H0.xbf"),
];

/// The original's model of each kind of crate.
const CRATES: [(CrateKind, &str); 4] = [
    (CrateKind::Money, "crates\\G_Crates_H0.xbf"),
    (CrateKind::Sardaukar, "crates\\Crate_IM_Sarcophahus_H0.xbf"),
    (CrateKind::Contaminator, "crates\\Crate_TL_Egg_H0.xbf"),
    (CrateKind::Infiltrator, "crates\\Crate_IX_Pod_H0.xbf"),
];

/// The construction yard each house's MCV deploys into.
const CONSTRUCTION_YARDS: [(Faction, &str); 3] = [
    (Faction::Atreides, "Buildings\\AT_conyard_H0.XbF"),
    (Faction::Harkonnen, "Buildings\\HK_conyard_H0.XbF"),
    (Faction::Ordos, "Buildings\\OR_conyard_H0.XBF"),
];

/// The folder of the buildings' and towers' models, which have damaged ones too.
const BUILDINGS: &str = "buildings\\";
/// What the name of a whole model ends with, and what those of its damaged ones do instead,
/// as it is damaged and then badly.
const WHOLE: &str = "_h0.";
const DAMAGED: [&str; 2] = ["_h1.", "_h2."];
/// The original's sidebar model, in its interface data in the cabinet, and the parts of it that
/// are the symbols on the build menu's tabs: a rifle, a tank and a building.
const UI_INDEX: &str = "\\data\\UI0001.RFH";
const UI_DATA: &str = "\\data\\UI0001.RFD";
const SIDEBAR: &str = "SIDEBAR\\SIDEBAR1.XBF";
/// The original's cursors in the interface's data, each for the look it gives the cursor:
/// models of their own, most lying on the ground under it.
const CURSORS: [(CursorLook, &str); 14] = [
    (CursorLook::Arrow, "CU_Pointer_H0"),
    (CursorLook::Attack, "CU_attack_H0"),
    (CursorLook::Heal, "CU_Repair_H0"),
    (CursorLook::Carry, "CU_PickUp_H0"),
    (CursorLook::Move, "CU_Move_H0"),
    (CursorLook::Deploy, "CU_Deploy_H0"),
    (CursorLook::Undeploy, "CU_Deploy_H0"),
    (CursorLook::Land, "CU_Enter_H0"),
    (CursorLook::Harvest, "CU_attack_H0"),
    (CursorLook::March, "CU_Move_Formation_H0"),
    (CursorLook::Board, "CU_Enter_H0"),
    (CursorLook::Sell, "CU_Sell_H0"),
    (CursorLook::Repair, "CU_Repair_H0"),
    (CursorLook::Wait, "CU_Wait_H0"),
];
const CURSOR_FOLDER: &str = "CURSORS\\";
/// The parts of the sidebar's model that are its frame: its back, then its glowing rims.
const SIDEBAR_FRAME: [&str; 2] = ["sidebar", "sidebar01"];

/// The original's lettering: the picture of its letters among the models' pictures, and the
/// list of which they are in the interface's data.
const FONT_PICTURE: &str = "Standard.tga";
const FONT_LIST: &str = "FONT\\STANDARD.MAP";
/// The symbols on the original's sidebar: those of its tabs and of its command buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symbol {
    Infantry,
    Vehicles,
    Buildings,
    Upgrades,
    Starport,
    Repair,
    Sell,
    Waypoint,
    Retreat,
    Guard,
}

/// The parts of the sidebar's model each symbol is made of.
const SYMBOLS: [(Symbol, &[&str]); 10] = [
    (Symbol::Infantry, &["infantrybody"]),
    (Symbol::Vehicles, &["vehiclebody"]),
    (Symbol::Buildings, &["buildingbody"]),
    (Symbol::Upgrades, &["upgradebuildings", "upgradearrow"]),
    (Symbol::Starport, &["Star01", "rocket"]),
    (Symbol::Repair, &["spanner"]),
    (Symbol::Sell, &["dollar"]),
    (Symbol::Waypoint, &["concreteicon"]),
    (Symbol::Retreat, &["flagpole", "flag"]),
    (Symbol::Guard, &["sheild"]),
];

/// What the name of its model as it is destroyed ends with.
const DESTROYED: &str = "_h3.";
/// The run of a model's animation as it is destroyed.
const EXPLODE: &str = "Explode";
/// How much of its health a building has left when it shows each of its damaged models. The
/// disc does not say; these are the shares Westwood's games turn a health bar yellow and red at.
const DAMAGED_BELOW: [f32; 2] = [0.5, 0.25];

/// The houses' and mini-houses' logos among the pictures.
pub const LOGOS: [&str; 8] = [
    "atlogo", "hklogo", "orlogo", "frlogo", "imlogo", "ixlogo", "tllogo", "gulogo",
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

pub(super) const IDENTITY: [[f32; 4]; 4] = [
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
    /// A building's or a tower's models as it is damaged, then badly, as the disc has them,
    /// by the name of its whole one.
    damaged: HashMap<&'static str, [Option<Shape>; 2]>,
    /// What a building or a tower sounds like as it is destroyed, by the name of its whole
    /// model: what its ruin's model plays as it goes up.
    ruins: HashMap<&'static str, Vec<(f32, String)>>,
    /// Which of the pictures each house's logo is, by its name.
    logos: HashMap<&'static str, usize>,
    /// Which of the pictures are the frames of the shimmer an invisible unit shows its own
    /// side, one after another.
    stealth: Vec<usize>,
    pictures: Vec<u8>,
    /// How many pictures the models, spice and build buttons have, after which a map's own come.
    layers: usize,
    /// Which of the pictures each of `ICONS` is, lit and grey, if the disc has it.
    icons: Vec<Option<(usize, usize)>>,
    /// Which of the pictures each of `UPGRADE_ICONS` is, a house's pad and then its yard.
    upgrade_icons: Vec<[Option<(usize, usize)>; 2]>,
    /// The main menu's scene, if the disc has it.
    menu: Option<frontend::Scene>,
    /// The letters text is drawn with, if the disc has them.
    lettering: Option<Lettering>,
    /// The cursor of each of `CURSORS`, if the disc has it.
    cursors: Vec<Option<frontend::Scene>>,
    /// The sidebar's frame, if the disc has it.
    sidebar_frame: Option<frontend::Scene>,
    /// Each of `SYMBOLS` as the sidebar's model draws it, glowing, if the disc has it.
    symbol_scenes: Vec<Option<frontend::Scene>>,
    /// The symbol of each of `SYMBOLS`, as flat triangles in a square from -1 to 1 across and
    /// down: none for one the disc has not.
    symbols: Vec<Vec<[(f32, f32); 3]>>,
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

    /// Lays the patches of the cell at column `u` and row `v`, each corner shaded by the fog
    /// where it lies, as the ground under it is.
    fn lay(
        &self,
        decals: &mut Vec<WorldVertex>,
        terrain: &Terrain,
        (u, v): (usize, usize),
        fog: Option<&Fog>,
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
                let (x, z) = (mx + cx, mz + cz);
                decals.push(WorldVertex {
                    x,
                    y,
                    z,
                    normal: [0.0, 1.0, 0.0],
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    seen: 1.0,
                    level: 1.0,
                    stain: [0.0; 3],
                    picture: [pu, pv, self.layer as f32],
                    team: [0.0; 3],
                    fog: battlefield::fog_at(fog, x, z),
                });
            }
        }
    }
}

/// A map's ground as the original shapes it: its pieces, and the pictures it is painted with,
/// to be given to the backend after the models'.
struct Land {
    pieces: Vec<Piece>,
    /// How high its sea lies, if it has one: the lowest of it.
    sea: Option<f32>,
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
    /// Whether it is the sea's surface.
    sea: bool,
}

/// The pictures the original paints its seas with: Caladan's, and the one between the ice
/// sheets of Draconis.
const SEAS: [&str; 2] = ["sea.tga", "icesea.tga"];

/// How deep the sea is drawn under its surface: the disc's maps have no bed under it.
const SEA_DEPTH: f32 = 45.25;

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
    /// The sounds its animation plays, each at its frame, by its section in the sound tables.
    sounds: Vec<(usize, String)>,
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
    /// Whether it is what swings a gun round on its own, as far as the unit has swung it.
    swivel: bool,
    /// Which of the model's weapon ends it is, for a part that marks one.
    muzzle: Option<usize>,
    /// The faces drawn over what lies behind them, each corner at `uv` on its picture.
    veils: Vec<Veil>,
    /// Whether it is a searchlight's beam, whose far end lights the ground.
    beam: bool,
    /// Where in it a light shines, for a box that only marks where, as the starports' landing
    /// lights and the Atreides factory's lights.
    glint: Option<[f32; 3]>,
    /// Whether it is the gun a soldier sets up, standing apart from the one he carries: it
    /// holds the turret of the gun he fires once dug in (`::1`) and stays put but for its recoil,
    /// and shows only once he is, as a mortar's `Mortorgun01`.
    set_up: bool,
    /// For one of the pads a refinery's model slides out as it is upgraded, which: 1 for
    /// `SmallPad01`, played by the sequence `Refinery Pad 1`, out with the first upgrade.
    pad: Option<u8>,
}

/// A face drawn over what lies behind it: a light's glow, adding its picture, or a
/// see-through one, laid over it as much as its picture shows.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Veil {
    points: [[f32; 3]; 3],
    normals: [[f32; 3]; 3],
    layer: usize,
    uv: [[f32; 2]; 3],
    glows: bool,
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
    /// What is seen through as much as its alpha says, the backend's `layer`th picture.
    SeeThrough {
        image: Image,
        layer: usize,
    },
    Missing,
}

impl Picture {
    /// Which of those given to the backend it is, if it is given.
    fn layer(&self) -> Option<usize> {
        match self {
            Picture::Painted { layer, .. }
            | Picture::Glow { layer, .. }
            | Picture::SeeThrough { layer, .. } => Some(*layer),
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

/// How high the sea of the map of this number lies, once its ground is read, if it has one.
pub(super) fn sea_level(map: usize) -> Option<f32> {
    land(map)?.sea
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
    let seas: Vec<bool> = model
        .textures
        .iter()
        .map(|texture| SEAS.iter().any(|sea| texture.eq_ignore_ascii_case(sea)))
        .collect();
    let mut pieces = Vec::new();
    for object in &model.objects {
        gather_land(object, &IDENTITY, &used, &seas, &mut pieces);
    }
    // The walls it hangs down from its edges, all one point of their picture, show past the
    // map as its edge stretched far down: they are left out.
    let extent = battlefield::terrain(map).extent();
    for piece in &mut pieces {
        piece.faces.retain(|face| !hangs_from_edge(face, extent));
    }
    let sea = pieces
        .iter()
        .flat_map(|piece| &piece.faces)
        .filter(|face| face.sea)
        .flat_map(|face| face.points.map(|[_, y, _]| y))
        .min_by(f32::total_cmp);
    let cells = battlefield::terrain(map).cells();
    let wash = crate::maps::file(map, "test.CPT").and_then(|data| xbf::wash(&data, cells));
    Some(Land {
        pieces,
        sea,
        pictures: images,
        wash,
    })
}

/// Whether a face of a map's ground is in a wall down from one of its edges, of a map that
/// reaches this far along x and along z: all its corners on that edge.
fn hangs_from_edge(face: &LandFace, (right, bottom): (f32, f32)) -> bool {
    const NEAR: f32 = 0.5;
    let all = |on: &dyn Fn([f32; 3]) -> bool| face.points.iter().all(|&point| on(point));
    [(0, 0.0), (0, right), (2, 0.0), (2, bottom)]
        .iter()
        .any(|&(axis, at)| all(&|point| (point[axis] - at).abs() < NEAR))
}

/// Adds the faces of a piece of a map's ground and what is in it, placed by `above`. The
/// original's rows run the other way along z from ours.
fn gather_land(
    object: &Object,
    above: &[[f32; 4]; 4],
    pictures: &[Picture],
    seas: &[bool],
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
            sea: face.texture.is_some_and(|n| seas.get(n) == Some(&true)),
        });
    }
    if !faces.is_empty() {
        pieces.push(Piece { covers, faces });
    }
    for child in &object.children {
        gather_land(child, &world, pictures, seas, pieces);
    }
}

/// Adds a face of a map's ground to the ground, or if it is the sea's surface to the water,
/// under which the color of its picture lies deep.
fn lay(
    face: &LandFace,
    fog: Option<&Fog>,
    ground: &mut Vec<WorldVertex>,
    water: &mut Vec<WorldVertex>,
) {
    let list = if face.sea { water } else { ground };
    for n in 0..3 {
        let [x, y, z] = face.points[n];
        let [r, g, b] = face.color;
        list.push(WorldVertex {
            x,
            y,
            z,
            normal: face.normals[n],
            r,
            g,
            b,
            seen: 1.0,
            level: if face.sea { -SEA_DEPTH } else { y },
            stain: [0.0; 3],
            picture: if face.sea {
                NO_PICTURE
            } else {
                face.picture[n]
            },
            team: [0.0; 3],
            fog: battlefield::fog_at(fog, x, z),
        });
    }
}

/// Draws the ground of the map of this number as the original shapes it, as much of it as
/// the view could show, if it can.
pub(super) fn draw_land(
    ground: &mut Vec<WorldVertex>,
    water: &mut Vec<WorldVertex>,
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
            lay(face, fog, ground, water);
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
            spice.lay(decals, terrain, (u, v), fog);
        }
    }
}

fn read_shapes(disc: &Path) -> Option<Original> {
    let index = archive::from_cabinet(&disc.join(CABINET), INDEX)?;
    let archive = Archive::open(&index, &disc.join(DATA))?;
    let mut pictures = HashMap::new();
    let names = MODELS.iter().map(|&(_, _, name)| name);
    let names = names.chain(CONSTRUCTION_YARDS.iter().map(|&(_, name)| name));
    let names = names.chain(WALLS.iter().flat_map(|(_, pieces)| pieces.iter().copied()));
    let names = names.chain([TORNADO, FRIGATE]);
    let names = names.chain(SCENERY.iter().map(|&(_, name)| name));
    let names = names.chain(EXTRAS.iter().map(|&(_, name)| name));
    let names = names.chain(CRATES.iter().map(|&(_, name)| name));
    let shapes: HashMap<_, _> = names
        .filter_map(|name| Some((name, read_shape(&archive, name, &mut pictures)?)))
        .collect();
    let variant = |name: &str, of: &str| {
        let lower = name.to_lowercase();
        let at = lower.rfind(WHOLE)?;
        Some(format!(
            "{}{of}{}",
            &lower[..at],
            &lower[at + WHOLE.len()..]
        ))
    };
    let buildings: Vec<&'static str> = shapes
        .keys()
        .copied()
        .filter(|name| name.to_lowercase().starts_with(BUILDINGS))
        .collect();
    let damaged = buildings
        .iter()
        .map(|&name| {
            let shape = |of: &str| read_shape(&archive, &variant(name, of)?, &mut pictures);
            (name, DAMAGED.map(shape))
        })
        .collect();
    let ruins = buildings
        .iter()
        .filter_map(|&name| {
            let model = xbf::parse(&archive.read(&variant(name, DESTROYED)?)?)?;
            let explode = model.sequences.iter().find(|s| s.name == EXPLODE)?;
            Some((name, sounds_in(&model.sounds, &explode.ranges)))
        })
        .collect();
    let mut given: Vec<(usize, Vec<u8>)> = pictures
        .values()
        .filter_map(|picture| match picture {
            Picture::Painted {
                image, team, layer, ..
            } => Some((*layer, square(image, *team))),
            Picture::Glow { image, layer } | Picture::SeeThrough { image, layer } => {
                Some((*layer, see_through(image)))
            }
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
    let logos = LOGOS
        .iter()
        .filter_map(|&name| {
            let image = xbf::tga(&archive.read(&format!("{TEXTURES}{name}.tga"))?)?;
            pictures.extend(see_through(&image));
            layers += 1;
            Some((name, layers - 1))
        })
        .collect();
    let stealth = (0..STEALTH_FRAMES)
        .map_while(|frame| {
            let name = format!("{TEXTURES}hackedstealth{frame}.tga");
            let image = xbf::tga(&archive.read(&name)?)?;
            pictures.extend(see_through(&image));
            layers += 1;
            Some(layers - 1)
        })
        .collect();
    let mut give = |name: &str| {
        let image = xbf::tga(&archive.read(&format!("{TEXTURES}{name}.tga"))?)?;
        pictures.extend(square(&image, false));
        layers += 1;
        Some(layers - 1)
    };
    let icons = ICONS
        .iter()
        .map(|&(_, _, lit, grey)| {
            let lit = give(lit)?;
            Some((lit, give(grey).unwrap_or(lit)))
        })
        .collect();
    let upgrade_icons = UPGRADE_ICONS
        .iter()
        .map(|(_, pictures)| {
            pictures.map(|(lit, grey)| {
                let lit = give(lit)?;
                Some((lit, give(grey).unwrap_or(lit)))
            })
        })
        .collect();
    let symbols = read_symbols(disc).unwrap_or_default();
    let menu = read_interface(disc, frontend::SCENE)
        .and_then(|model| frontend::read(&model, &archive, &mut pictures, &mut layers));
    let lettering = archive
        .read(&format!("{TEXTURES}{FONT_PICTURE}"))
        .and_then(|bytes| xbf::tga(&bytes))
        .zip(read_interface(disc, FONT_LIST))
        .and_then(|(image, listed)| lettering::read(&image, &listed, &mut pictures, &mut layers));
    let cursors = CURSORS
        .iter()
        .map(|(_, name)| {
            let model = read_interface(disc, &format!("{CURSOR_FOLDER}{name}.xbf"))?;
            frontend::read(&model, &archive, &mut pictures, &mut layers)
        })
        .collect();
    let sidebar = read_interface(disc, SIDEBAR);
    let sidebar_frame = sidebar.as_ref().and_then(|model| {
        frontend::read_parts(model, &SIDEBAR_FRAME, &archive, &mut pictures, &mut layers)
    });
    let symbol_scenes = SYMBOLS
        .iter()
        .map(|(_, parts)| {
            let model = sidebar.as_ref()?;
            frontend::read_parts(model, parts, &archive, &mut pictures, &mut layers)
        })
        .collect();
    (!shapes.is_empty()).then_some(Original {
        shapes,
        damaged,
        ruins,
        symbols,
        logos,
        stealth,
        pictures,
        layers,
        icons,
        upgrade_icons,
        menu,
        lettering,
        cursors,
        sidebar_frame,
        symbol_scenes,
        archive,
        spice,
    })
}

/// Draws the original's cursor of this look at `at`, `scale` pixels to its unit, as it is
/// this many seconds on, once the disc's are read. Returns whether it was drawn.
pub(super) fn draw_cursor(
    vertices: &mut Vec<Vertex>,
    look: CursorLook,
    at: (f32, f32),
    scale: f32,
    time: f32,
) -> bool {
    let Some(original) = ORIGINAL.get() else {
        return false;
    };
    let found = CURSORS.iter().position(|&(of, _)| of == look);
    let Some(Some(cursor)) = found.and_then(|n| original.cursors.get(n)) else {
        return false;
    };
    cursor.draw_flat(vertices, at, scale, time);
    true
}

/// Draws the original's sidebar frame stretched over a box (x, y, w, h), once the disc's is
/// read.
pub(super) fn draw_sidebar_frame(vertices: &mut Vec<Vertex>, rect: (f32, f32, f32, f32)) {
    if let Some(frame) = ORIGINAL.get().and_then(|o| o.sidebar_frame.as_ref()) {
        frame.draw_fitted(vertices, rect);
    }
}

/// The original's lettering, once the disc's is read.
pub(super) fn lettering() -> Option<&'static Lettering> {
    ORIGINAL.get()?.lettering.as_ref()
}

/// A file out of the disc's interface archive.
fn read_interface(disc: &Path, name: &str) -> Option<Vec<u8>> {
    let cabinet = disc.join(CABINET);
    let index = archive::from_cabinet(&cabinet, UI_INDEX)?;
    let data = archive::from_cabinet(&cabinet, UI_DATA)?;
    archive::read_in(&index, &data, name)
}

/// Draws the original's main menu scene behind the menus, as it is this many seconds on, once
/// the disc's is read. Returns whether it was drawn.
pub(super) fn draw_menu(vertices: &mut Vec<Vertex>, screen: (f32, f32), time: f32) -> bool {
    let scene = ORIGINAL.get().and_then(|original| original.menu.as_ref());
    scene.is_some_and(|scene| {
        scene.draw(vertices, screen, time);
        true
    })
}

/// The sidebar's symbols out of its model, each in the order of `SYMBOLS`. The model faces
/// along x, its right along y and its up along z.
fn read_symbols(disc: &Path) -> Option<Vec<Vec<[(f32, f32); 3]>>> {
    let model = xbf::parse(&read_interface(disc, SIDEBAR)?)?;
    fn find(object: &Object, above: &[[f32; 4]; 4], name: &str) -> Option<Vec<[[f32; 3]; 3]>> {
        let world = times(&object.transform, above);
        if object.name.eq_ignore_ascii_case(name) {
            let corner = |c: usize| object.vertices.get(c).map(|v| point(&world, v.position));
            return object
                .faces
                .iter()
                .map(|face| {
                    Some([
                        corner(face.corners[0])?,
                        corner(face.corners[1])?,
                        corner(face.corners[2])?,
                    ])
                })
                .collect();
        }
        object
            .children
            .iter()
            .find_map(|child| find(child, &world, name))
    }
    let symbols = SYMBOLS.iter().map(|&(_, parts)| {
        let faces: Vec<[[f32; 3]; 3]> = parts
            .iter()
            .filter_map(|part| model.objects.iter().find_map(|o| find(o, &IDENTITY, part)))
            .flatten()
            .collect();
        let (mut low, mut high) = ([f32::MAX; 2], [f32::MIN; 2]);
        for &[_, y, z] in faces.iter().flatten() {
            (low, high) = (
                [low[0].min(y), low[1].min(z)],
                [high[0].max(y), high[1].max(z)],
            );
        }
        let middle = [(low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0];
        let half = ((high[0] - low[0]).max(high[1] - low[1]) / 2.0).max(f32::EPSILON);
        faces
            .iter()
            .map(|face| face.map(|[_, y, z]| ((y - middle[0]) / half, (middle[1] - z) / half)))
            .collect()
    });
    Some(symbols.collect())
}

/// A symbol of the original's sidebar: flat triangles in a square from -1 to 1 across and
/// down, once the disc's are read.
/// Draws one of the sidebar's symbols as the original does, glowing in its own pictures, as
/// big as fits a square `size` wide with its middle at `at`, as bright as `tint`. Returns
/// whether it was drawn.
pub(super) fn draw_symbol_icon(
    vertices: &mut Vec<Vertex>,
    symbol: Symbol,
    at: (f32, f32),
    size: f32,
    tint: f32,
) -> bool {
    let at_symbol = SYMBOLS.iter().position(|&(of, _)| of == symbol);
    let scene = ORIGINAL
        .get()
        .zip(at_symbol)
        .and_then(|(original, at)| original.symbol_scenes.get(at)?.as_ref());
    scene.is_some_and(|scene| {
        scene.draw_icon(vertices, at, size, tint);
        true
    })
}

pub fn symbol(symbol: Symbol) -> Option<&'static [[(f32, f32); 3]]> {
    let at = SYMBOLS.iter().position(|&(of, _)| of == symbol)?;
    ORIGINAL
        .get()?
        .symbols
        .get(at)
        .map(Vec::as_slice)
        .filter(|faces| !faces.is_empty())
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
        sounds: model.sounds,
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
        Some(image) if marks.contains('@') => Picture::SeeThrough { image, layer },
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
    // A light's glows, and a glow whose picture slides over it, as a windtrap's wind does;
    // and what is seen through, unless it only shows when something has happened to the
    // unit, as the mark of a leech in it does.
    let flows = shape.slides.iter().any(|slide| slide.part == object.name);
    let lit = is_light(&object.name) || flows;
    let shown = !object.name.contains(CONDITION_MARK);
    let veils = object
        .faces
        .iter()
        .filter_map(|face| {
            let (layer, glows) = match face.texture.and_then(|n| pictures.get(n).copied()) {
                Some(Picture::Glow { layer, .. }) if lit => (*layer, true),
                // A light kept out of sight as a mark, painted with a picture not marked to
                // glow, shows it unlit and covering what is behind, as a windtrap's front and
                // back panels do; a box that only marks where a light is, `#`, shows nothing.
                Some(Picture::Painted { layer, .. })
                    if lit && object.name.starts_with(LIGHT_FACE_MARK) =>
                {
                    (*layer, false)
                }
                Some(Picture::SeeThrough { layer, .. }) if shown => (*layer, false),
                _ => return None,
            };
            Some(Veil {
                points: face.corners.map(|corner| object.vertices[corner].position),
                normals: face.corners.map(|corner| object.vertices[corner].normal),
                layer,
                uv: face.uv,
                glows,
            })
        })
        .collect();
    shape.parts.push(Part {
        faces,
        veils,
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
        swivel: object.name == SWIVEL,
        beam: object.name.to_ascii_lowercase().contains(SEARCHLIGHT),
        glint: glint_of(object),
        set_up: object
            .children
            .iter()
            .any(|child| child.name.contains(DUG_IN_TURRET))
            && object.animation.as_ref().is_some_and(|animation| {
                let [x, y, z, _] = animation.poses.first().map_or([0.0; 4], |pose| pose[3]);
                animation.poses.iter().all(|pose| {
                    let [px, py, pz, _] = pose[3];
                    (px - x).hypot(py - y).hypot(pz - z) <= SET_UP_GUN_STILL
                })
            }),
        muzzle: object
            .name
            .strip_prefix(MUZZLE_MARK)
            .map(|rest| rest.trim_start_matches(|c: char| !c.is_ascii_digit()))
            .and_then(|rest| {
                let digits =
                    rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
                rest[..digits].parse().ok()
            }),
        pad: object
            .name
            .split_once(SLIDING_PAD)
            .and_then(|(_, number)| number.parse().ok()),
    });
    let this = shape.parts.len() - 1;
    for child in &object.children {
        gather(child, Some(this), &world, head, pictures, shape);
    }
}

/// What a light's mark that shows its faces is named with, as a windtrap's panels; a light's
/// mark named with `GLINT_MARK` is only a box where it shines.
const LIGHT_FACE_MARK: char = '?';
const GLINT_MARK: char = '#';

/// The middle of a box that marks where a light shines, as `#light01`, in the object's own
/// space: none for anything else, or a mark with nothing to show where.
fn glint_of(object: &Object) -> Option<[f32; 3]> {
    if !object.name.starts_with(GLINT_MARK) || !is_light(&object.name) {
        return None;
    }
    let count = object.vertices.len() as f32;
    (count > 0.0).then(|| {
        std::array::from_fn(|i| object.vertices.iter().map(|v| v.position[i]).sum::<f32>() / count)
    })
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
        Some(Picture::Glow { .. } | Picture::SeeThrough { .. }) => return None,
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

/// The model the original has for the unit, if it has been read: a building's or a tower's
/// as damaged as it is.
fn shape_of(unit: &Unit) -> Option<&'static Shape> {
    let name = model_name(unit)?;
    let original = ORIGINAL.get()?;
    let share = unit.health / unit.max_health();
    let damaged = original
        .damaged
        .get(name)
        .and_then(|shapes| as_damaged(share, shapes));
    damaged.or_else(|| original.shapes.get(name))
}

/// The name of the original's whole model of a unit.
fn model_name(unit: &Unit) -> Option<&'static str> {
    if unit.role == Role::Wall {
        let (piece, _) = WallPiece::of(unit.wall_joins);
        WALLS
            .iter()
            .find(|&&(house, _)| house == unit.house)
            .map(|(_, pieces)| pieces[piece as usize])
    } else if unit.role == Role::Mcv && unit.deployed {
        CONSTRUCTION_YARDS
            .iter()
            .find(|&&(house, _)| house == unit.house)
            .map(|&(_, name)| name)
    } else if let Role::Extra(extra) = unit.role {
        EXTRAS
            .iter()
            .find(|&&(rules_name, _)| rules_name == extra.rules_name())
            .map(|&(_, name)| name)
    } else if let Role::Prop(Prop::Scenery(scenery)) = unit.role {
        SCENERY
            .iter()
            .find(|&&(rules_name, _)| rules_name == scenery.rules_name())
            .map(|&(_, name)| name)
    } else {
        MODELS
            .iter()
            .find(|&&(role, house, _)| role == unit.role && house.is_none_or(|h| h == unit.house))
            .map(|&(_, _, name)| name)
    }
}

/// The sounds a run of a model's animation plays, played from its start: each by its section
/// in the sound tables, and how many seconds into the run it is.
fn sounds_in(sounds: &[(usize, String)], ranges: &[(usize, usize)]) -> Vec<(f32, String)> {
    let mut into = 0;
    let mut heard = Vec::new();
    for &(first, last) in ranges {
        heard.extend(
            sounds
                .iter()
                .filter(|&&(frame, _)| (first..=last).contains(&frame))
                .map(|(frame, name)| {
                    (
                        (into + frame - first) as f32 / FRAMES_PER_SECOND,
                        name.clone(),
                    )
                }),
        );
        into += length((first, last));
    }
    heard
}

/// What the original's model of a unit plays as it is heard: the run it fires with, as
/// `frame` picks it, or the run it dies with, a soldier's as it was wounded, and a building's
/// or a tower's as its ruin goes up. Each sound, by its section in the sound tables, comes
/// with how many seconds after it is heard it plays.
pub fn noise_sounds(noise: &Noise) -> Vec<(f32, String)> {
    let unit = &noise.unit;
    let Some((original, name)) = ORIGINAL.get().zip(model_name(unit)) else {
        return Vec::new();
    };
    if let Made::Died { .. } = noise.made
        && matches!(unit.kind, UnitKind::Building | UnitKind::Tower)
    {
        return original.ruins.get(name).cloned().unwrap_or_default();
    }
    let Some(shape) = original.shapes.get(name) else {
        return Vec::new();
    };
    let find = |names: &[&str]| {
        names.iter().find_map(|&name| {
            shape
                .sequences
                .iter()
                .find(|sequence| sequence.name == name)
                .map(|sequence| sequence.ranges.as_slice())
        })
    };
    let run = match noise.made {
        Made::Fired { .. } if unit.deployed => find(&["Deployed Fire", "Fire 1"]),
        Made::Fired { second: true } => find(&["Fire 1", "Fire 0"]),
        Made::Fired { second: false } => find(&["Fire 0"]),
        Made::Died { wound, second } if unit.kind == UnitKind::Infantry => death_run(
            shape,
            &Body {
                unit: *unit,
                wound,
                second,
                age: 0.0,
            },
        ),
        Made::Died { .. } => find(&[EXPLODE]),
    };
    run.map_or_else(Vec::new, |ranges| sounds_in(&shape.sounds, ranges))
}

/// Of a building's damaged models, the one for this share of its health left: the worst it
/// is down to that there is, or none while it is still whole enough.
fn as_damaged<T>(share: f32, shapes: &[Option<T>; 2]) -> Option<&T> {
    DAMAGED_BELOW
        .iter()
        .zip(shapes)
        .rev()
        .find_map(|(&below, shape)| shape.as_ref().filter(|_| share < below))
}

/// Which of the pictures given to the backend is the logo of this name, one of `LOGOS`, once
/// the disc's pictures have been read.
pub fn logo(name: &str) -> Option<usize> {
    ORIGINAL.get()?.logos.get(name).copied()
}

/// How many frames the shimmer of an invisible unit has.
const STEALTH_FRAMES: usize = 8;

/// Which of the pictures given to the backend the shimmer of an invisible unit is at this time
/// of the battle, going round its frames one each update, once the disc's pictures are read.
pub fn stealth(time: f32) -> Option<usize> {
    let frames = &ORIGINAL.get()?.stealth;
    frames
        .get(looped(
            &[(0, frames.len())],
            (time * FRAMES_PER_SECOND) as usize,
        ))
        .copied()
}

/// Which of the pictures given to the backend is the build button of this role for this
/// house, grey while it cannot be made, once the disc's are read.
pub fn icon(role: Role, house: Faction, grey: bool) -> Option<usize> {
    let (lit, dim) = ORIGINAL.get()?.icons[icon_at(role, house)?]?;
    Some(if grey { dim } else { lit })
}

/// Which of the pictures is the Upgrades menu's button for a building of this role of this
/// house: a refinery's is the pad it gains, a deployed MCV's the construction yard, and any
/// other building's its own build button.
pub fn upgrade_icon(role: Role, house: Faction, grey: bool) -> Option<usize> {
    let which = match role {
        Role::Refinery => 0,
        Role::Mcv => 1,
        _ => return icon(role, house, grey),
    };
    let at = UPGRADE_ICONS.iter().position(|&(of, _)| of == house)?;
    let (lit, dim) = ORIGINAL.get()?.upgrade_icons[at][which]?;
    Some(if grey { dim } else { lit })
}

/// Which of `ICONS` is the build button of this role for this house.
fn icon_at(role: Role, house: Faction) -> Option<usize> {
    ICONS
        .iter()
        .position(|&(of, only, ..)| of == role && only.is_none_or(|h| h == house))
}

/// Whether the disc is known to have a build button for this role of this house.
#[cfg(test)]
pub fn names_icon(role: Role, house: Faction) -> bool {
    icon_at(role, house).is_some()
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
        _ if unit.cheering => &["Win", "Stationary"],
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

/// What the name of a part holds that shows only when something has happened to the unit.
const CONDITION_MARK: char = '{';

/// What the name of a part of a refinery's model holds before the number of its pad, for the
/// pads it slides out as it is upgraded; and what the sequence that slides one out is called,
/// before the same number.
const SLIDING_PAD: &str = "SmallPad";
const PAD_SLIDES_OUT: &str = "Refinery Pad ";

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
        // Turned so that its piece joins the walls beside it. As it is made, its +x lies
        // along the world's +x turned a quarter back.
        UnitKind::Building if unit.role == Role::Wall => {
            let (_, turns) = WallPiece::of(unit.wall_joins);
            let heading = (turns as f32 - 1.0) * FRAC_PI_2;
            let body = Place::turned([unit.x, unit.y, unit.z], heading);
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
        // A machine whose weapon swings round on its own, as a Mongoose's launcher, has its
        // head turned so.
        UnitKind::Vehicle | UnitKind::Infantry => {
            let body = Place::level(unit);
            let head = body.moved(pivot);
            match unit.role.turret_swing() {
                Some(_) => (body, head.yawed(unit.gun_turn)),
                None => (body, head),
            }
        }
    }
}

/// Where each part of the model is within it at a frame, the barrel lifted by `pitch` and a
/// gun that swings on its own swung round by `swing`.
fn pose(shape: &Shape, pitch: f32, swing: f32, frame: usize) -> Vec<[[f32; 4]; 4]> {
    pose_by(shape, pitch, swing, |_| frame)
}

/// Where each part of the model is, each at its own frame of the animation.
fn pose_by(
    shape: &Shape,
    pitch: f32,
    swing: f32,
    frame: impl Fn(&Part) -> usize,
) -> Vec<[[f32; 4]; 4]> {
    let (sin, cos) = pitch.sin_cos();
    // Turned about its own across, so that its front, along -z, comes up.
    let lifted = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, cos, sin, 0.0],
        [0.0, -sin, cos, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    // Turned about the up the way the unit turns, which in the model's axes is the other way.
    let (sin, cos) = (-swing).sin_cos();
    let swung = [
        [cos, 0.0, -sin, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [sin, 0.0, cos, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut worlds: Vec<[[f32; 4]; 4]> = Vec::with_capacity(shape.parts.len());
    for part in &shape.parts {
        let above = part.parent.map_or(&IDENTITY, |parent| &worlds[parent]);
        let at = part.at(frame(part));
        let at = if part.lift { times(&lifted, &at) } else { at };
        let mut world = times(&at, above);
        // Swung about the model's up, staying where it is, and what sits in it with it.
        if part.swivel {
            let stays = world[3];
            world = times(&world, &swung);
            world[3] = stays;
        }
        worlds.push(world);
    }
    worlds
}

/// Where the ends of the weapons of the unit's model are on the battlefield, by their numbers,
/// if it has a model with any.
pub fn muzzles(unit: &Unit) -> Vec<(f32, f32, f32)> {
    shape_of(unit).map_or_else(Vec::new, |shape| ends_of(shape, unit))
}

/// Where the ends of the weapons of this model are on the battlefield, drawn for the unit.
fn ends_of(shape: &Shape, unit: &Unit) -> Vec<(f32, f32, f32)> {
    let (body, head) = places(shape, unit);
    let pivot = shape.pivot.unwrap_or_default();
    let worlds = pose(
        shape,
        unit.tube_pitch,
        unit.gun_turn,
        frame(shape, unit, 0.0),
    );
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

/// Where the lights its model marks with boxes shine on the battlefield, as the starports'
/// landing lights and the Atreides factory's do, at this time of the battle.
pub fn glints(unit: &Unit, time: f32) -> Vec<(f32, f32, f32)> {
    let Some(shape) = shape_of(unit) else {
        return Vec::new();
    };
    let (body, head) = places(shape, unit);
    let pivot = shape.pivot.unwrap_or_default();
    let worlds = pose(
        shape,
        unit.tube_pitch,
        unit.gun_turn,
        frame(shape, unit, time),
    );
    shape
        .parts
        .iter()
        .zip(&worlds)
        .filter_map(|(part, world)| {
            let place = to_place(point(world, part.glint?));
            let [x, y, z] = if part.head {
                head.point(std::array::from_fn(|i| place[i] - pivot[i]))
            } else {
                body.point(place)
            };
            Some((x, y, z))
        })
        .collect()
}

/// Where the searchlights of the unit's model throw their light, at this time of the battle:
/// the middle of the far end of each beam, as it is posed now, and how far out from it that
/// end spreads. None without the disc's model, or for a building.
pub fn searchlight_spots(unit: &Unit, time: f32) -> Vec<([f32; 3], f32)> {
    // A building's lights, such as a windtrap's glowing panels, light only themselves; but a
    // searchlight's beam, as the Harkonnen compound's of the Atreides' first battle, lights
    // where it points.
    let lights_ground = unit.kind != UnitKind::Building || unit.role == Role::Prop(Prop::Spotlight);
    let Some(shape) = shape_of(unit).filter(|_| lights_ground) else {
        return Vec::new();
    };
    // Between the frame it is at and the next, for the light to sweep on smoothly.
    let played = time * FRAMES_PER_SECOND;
    let (at, between) = (played.floor(), played.fract());
    // From the middle of a frame's time, to be sure of which frame it is.
    let at_frame = |n: f32| frame(shape, unit, (n + 0.5) / FRAMES_PER_SECOND);
    let now = beam_ends(shape, unit, at_frame(at));
    let next = beam_ends(shape, unit, at_frame(at + 1.0));
    now.into_iter()
        .zip(next)
        .map(|((at, spread), (to, spread_to))| {
            (
                std::array::from_fn(|i| at[i] + (to[i] - at[i]) * between),
                spread + (spread_to - spread) * between,
            )
        })
        .collect()
}

/// The middle of the far end of each beam of the unit's model at a frame, and how far out
/// from it its corners are.
fn beam_ends(shape: &Shape, unit: &Unit, frame: usize) -> Vec<([f32; 3], f32)> {
    let (body, head) = places(shape, unit);
    let pivot = shape.pivot.unwrap_or_default();
    let worlds = pose(shape, unit.tube_pitch, unit.gun_turn, frame);
    shape
        .parts
        .iter()
        .zip(&worlds)
        .filter(|(part, _)| part.beam)
        .filter_map(|(part, world)| {
            let (place, from) = if part.head {
                (head, pivot)
            } else {
                (body, [0.0; 3])
            };
            // Its lens is where it shines from, nearest the part's middle; its far end the
            // corners well out from there.
            let gap = |a: [f32; 3], b: [f32; 3]| (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f32>();
            let all = || part.veils.iter().flat_map(|veil| veil.points);
            let lens = all().min_by(|&a, &b| gap(a, [0.0; 3]).total_cmp(&gap(b, [0.0; 3])))?;
            let reach = all().map(|corner| gap(corner, lens)).fold(0.0, f32::max);
            let mut corners: Vec<[f32; 3]> = Vec::new();
            // Squared, half as far out is a quarter.
            for corner in all().filter(|&corner| gap(corner, lens) > reach / 4.0) {
                if !corners.contains(&corner) {
                    corners.push(corner);
                }
            }
            let corners: Vec<[f32; 3]> = corners
                .into_iter()
                .map(|corner| {
                    let at = to_place(point(world, corner));
                    place.point(std::array::from_fn(|i| at[i] - from[i]))
                })
                .collect();
            let count = corners.len() as f32;
            let middle: [f32; 3] =
                std::array::from_fn(|i| corners.iter().map(|c| c[i]).sum::<f32>() / count);
            let spread = corners
                .iter()
                .map(|c| (c[0] - middle[0]).hypot(c[2] - middle[2]))
                .fold(0.0, f32::max);
            Some((middle, spread))
        })
        .collect()
}

/// Draws the original's model of the unit in its house color, as it is at this time of the
/// battle, if there is one to draw.
pub(super) fn draw(canvas: &mut Canvas, unit: &Unit, team: [f32; 3], time: f32) -> bool {
    let Some(shape) = shape_of(unit) else {
        return false;
    };
    let (body, head) = places(shape, unit);
    let frame = frame(shape, unit, time);
    // Pictures slide all the time, but a machine's tracks only as it goes.
    let slid = if unit.kind == UnitKind::Vehicle && !unit.is_moving() {
        0.0
    } else {
        time * FRAMES_PER_SECOND
    };
    let posed = Posed {
        body,
        head,
        frame,
        slid,
        worlds: pose_by(shape, unit.tube_pitch, unit.gun_turn, |part| {
            part.pad.map_or(frame, |pad| pad_frame(shape, unit, pad))
        }),
        set_up: shows_set_up(&unit),
    };
    draw_posed(canvas, shape, &posed, team);
    true
}

/// The frame a refinery's pad is at that its model slides out as it is upgraded: tucked in at
/// the start of its sequence until its upgrade is done, then as far along as it has slid out,
/// and at the end once out.
fn pad_frame(shape: &Shape, unit: &Unit, pad: u8) -> usize {
    let name = format!("{PAD_SLIDES_OUT}{pad}");
    let Some(&(first, last)) = shape
        .sequences
        .iter()
        .find(|sequence| sequence.name == name)
        .and_then(|sequence| sequence.ranges.first())
    else {
        return 0;
    };
    let out = match unit.upgrades.cmp(&pad) {
        Ordering::Less => 0.0,
        Ordering::Equal => unit.popped,
        Ordering::Greater => 1.0,
    };
    first + ((last - first) as f32 * out).round() as usize
}

/// Whether what a unit's model has set up shows: only a soldier sets his gun down, and only
/// once dug in. A building's or machine's still gun, as a rocket turret's head, is always there.
fn shows_set_up(unit: &Unit) -> bool {
    unit.deployed || unit.kind != UnitKind::Infantry
}

/// A model as it is drawn at one time: where its body and its head are, the frame of its
/// animation it is at, how many frames on its pictures have slid, and where its parts are.
struct Posed {
    body: Place,
    head: Place,
    frame: usize,
    slid: f32,
    worlds: Vec<[[f32; 4]; 4]>,
    /// Whether what it sets up is set up, and shows.
    set_up: bool,
}

/// Draws a dead soldier with the original's model, as far into its death as it has got, and
/// once that is over lying as it ends; sinking into the sand as it goes. False if there is no
/// model to draw it with.
pub(super) fn draw_body(canvas: &mut Canvas, body: &Body, team: [f32; 3]) -> bool {
    let Some(shape) = shape_of(&body.unit) else {
        return false;
    };
    let mut unit = body.unit;
    unit.y -= (body.age - (BODY_SECONDS - BODY_SINK_SECONDS)).max(0.0) * BODY_SINKING;
    let (body_place, head) = places(shape, &unit);
    let frame = body_frame(shape, body);
    let posed = Posed {
        body: body_place,
        head,
        frame,
        slid: 0.0,
        worlds: pose(shape, unit.tube_pitch, unit.gun_turn, frame),
        set_up: shows_set_up(&unit),
    };
    draw_posed(canvas, shape, &posed, team);
    true
}

/// How fast a body sinks into the sand as it goes.
const BODY_SINKING: f32 = 18.1;

/// Which frame of its model a dead soldier is at: as far into the run of its death as it has
/// got, and its last frame once that is over.
fn body_frame(shape: &Shape, body: &Body) -> usize {
    let Some(ranges) = death_run(shape, body) else {
        return 0;
    };
    let played = (body.age * FRAMES_PER_SECOND) as usize;
    once(ranges, played)
        .or_else(|| ranges.last().map(|&(_, last)| last))
        .unwrap_or(0)
}

/// The run of its model a soldier dies with: the one for how it was wounded, or for dying dug
/// in if it was; the second of two if it is to die that way and has one; or else as if shot.
fn death_run<'a>(shape: &'a Shape, body: &Body) -> Option<&'a [(usize, usize)]> {
    let wound = match body.wound {
        Wound::Shot | Wound::BlownUp if body.unit.deployed => "Deployed Death",
        Wound::Shot => "Shot",
        Wound::BlownUp => "Blow Up",
        Wound::Burnt => "Burnt",
        Wound::Gassed => "Gassed",
        Wound::RunOver => "Run Over",
    };
    let (first, second) = (format!("{wound} 1"), format!("{wound} 2"));
    let names = [
        body.second.then_some(second.as_str()),
        Some(first.as_str()),
        Some("Shot 1"),
    ];
    names.into_iter().flatten().find_map(|name| {
        shape
            .sequences
            .iter()
            .find(|sequence| sequence.name == name)
            .map(|sequence| sequence.ranges.as_slice())
    })
}

/// Draws the original's tornado, as it is at this time of the battle, if it has been read:
/// coming up out of the sand, whirling, and sinking back into it as it dies away.
pub(super) fn draw_tornado(canvas: &mut Canvas, tornado: &Tornado, time: f32) -> bool {
    let Some(shape) = ORIGINAL
        .get()
        .and_then(|original| original.shapes.get(TORNADO))
    else {
        return false;
    };
    let frame = tornado_frame(shape, tornado.age(), tornado.left(), time);
    let (x, y, z) = tornado.at;
    let body = Place::turned([x, y, z], tornado.phase);
    let posed = Posed {
        body,
        head: body,
        frame,
        slid: time * FRAMES_PER_SECOND,
        worlds: pose(shape, 0.0, 0.0, frame),
        set_up: true,
    };
    draw_posed(canvas, shape, &posed, [0.0; 3]);
    true
}

/// Draws the original's frigate in its house color, as it is in its visit at this time of the
/// battle, if it has been read: its model's sequence for each stage, the ones between flying
/// and hovering, and landing and taking off, played out as far as the stage has got.
pub(super) fn draw_frigate(
    canvas: &mut Canvas,
    frigate: &Frigate,
    team: [f32; 3],
    time: f32,
) -> bool {
    let Some(shape) = ORIGINAL
        .get()
        .and_then(|original| original.shapes.get(FRIGATE))
    else {
        return false;
    };
    let frame = frigate_frame(shape, frigate, time);
    let body = Place::turned(frigate.at, frigate.heading);
    let posed = Posed {
        body,
        head: body,
        frame,
        slid: time * FRAMES_PER_SECOND,
        worlds: pose(shape, 0.0, 0.0, frame),
        set_up: true,
    };
    draw_posed(canvas, shape, &posed, team);
    true
}

/// Which frame of its model the frigate is at in its visit.
fn frigate_frame(shape: &Shape, frigate: &Frigate, time: f32) -> usize {
    let (name, looping) = match frigate.flight {
        Flight::Coming | Flight::Leaving => ("Fly", true),
        Flight::Slowing => ("FlyToHover", false),
        Flight::Descending | Flight::Climbing => ("Hover", true),
        Flight::Landing => ("Land", false),
        Flight::Down => ("Stationary", true),
        Flight::TakingOff => ("Takeoff", false),
        Flight::Speeding => ("HoverToFly", false),
    };
    let Some(ranges) = shape
        .sequences
        .iter()
        .find(|sequence| sequence.name == name)
        .map(|sequence| sequence.ranges.as_slice())
    else {
        return 0;
    };
    match (looping, ranges) {
        (true, _) => looped(ranges, (time * FRAMES_PER_SECOND) as usize),
        (false, &[(first, last), ..]) => first + ((last - first) as f32 * frigate.share()) as usize,
        _ => 0,
    }
}

/// Draws the original's model of a crate of this kind lying here, if it has been read.
pub(super) fn draw_crate(canvas: &mut Canvas, kind: CrateKind, at: [f32; 3], time: f32) -> bool {
    let Some(shape) = CRATES
        .iter()
        .find(|&&(of, _)| of == kind)
        .and_then(|&(_, name)| ORIGINAL.get()?.shapes.get(name))
    else {
        return false;
    };
    let played = (time * FRAMES_PER_SECOND) as usize;
    let frame = shape
        .sequences
        .iter()
        .find(|sequence| sequence.name == "Stationary")
        .map_or(0, |sequence| looped(&sequence.ranges, played));
    let body = Place::turned(at, 0.0);
    let posed = Posed {
        body,
        head: body,
        frame,
        slid: time * FRAMES_PER_SECOND,
        worlds: pose(shape, 0.0, 0.0, frame),
        set_up: true,
    };
    draw_posed(canvas, shape, &posed, [0.0; 3]);
    true
}

/// Which frame of its model a tornado is at, this many seconds since it rose and before it is
/// gone: rising for as long as its "Surface" plays, then sinking for as long as its "Sink"
/// does, and in between going round its "Move" at this time of the battle.
fn tornado_frame(shape: &Shape, age: f32, left: f32, time: f32) -> usize {
    let find = |name: &str| {
        shape
            .sequences
            .iter()
            .find(|sequence| sequence.name == name)
            .map(|sequence| sequence.ranges.as_slice())
    };
    let frames = |seconds: f32| (seconds * FRAMES_PER_SECOND) as usize;
    if let Some(ranges) = find("Sink") {
        let total: usize = ranges.iter().copied().map(length).sum();
        // Its last frame as it goes.
        if frames(left) < total
            && let Some(frame) = once(ranges, (total - frames(left)).min(total - 1))
        {
            return frame;
        }
    }
    if let Some(frame) = find("Surface").and_then(|ranges| once(ranges, frames(age))) {
        return frame;
    }
    find("Move").map_or(0, |ranges| looped(ranges, frames(time)))
}

/// Draws a posed model in its house color.
fn draw_posed(canvas: &mut Canvas, shape: &Shape, posed: &Posed, team: [f32; 3]) {
    let Posed {
        body,
        head,
        frame,
        slid,
        ref worlds,
        set_up,
    } = *posed;
    let pivot = shape.pivot.unwrap_or_default();
    let mut veils = Vec::new();
    let mut model = Model::new(canvas, body);
    // A part shows if what it is in does, a set-up gun only once set up.
    let mut shown: Vec<bool> = Vec::with_capacity(shape.parts.len());
    for part in &shape.parts {
        let within = part.parent.is_none_or(|parent| shown[parent]);
        shown.push(within && (set_up || !part.set_up));
    }
    for ((part, world), _) in shape
        .parts
        .iter()
        .zip(worlds)
        .zip(&shown)
        .filter(|(_, shown)| **shown)
    {
        let (mut model, from) = if part.head {
            (model.at(head), pivot)
        } else {
            (model.at(body), [0.0; 3])
        };
        // Only how far into its picture it has slid: a whole picture on looks the same.
        let slide = part.slide.map(|step| (step * slid).fract());
        let place = if part.head { head } else { body };
        for veil in &part.veils {
            let points = veil.points.map(|p| {
                let at = to_place(point(world, p));
                std::array::from_fn(|i| at[i] - from[i])
            });
            // What is seen through shows only its front, as a tornado's funnel does; a glow
            // shows from both sides.
            let normals = veil.normals.map(|n| to_place(normal(world, n)));
            if !veil.glows && !model.sees_front(points, normals) {
                continue;
            }
            veils.extend((0..3).map(|n| {
                let [x, y, z] = place.point(points[n]);
                let [u, v] = veil.uv[n];
                WorldVertex {
                    picture: [u + slide[0], v + slide[1], veil.layer as f32],
                    level: if veil.glows { GLOW } else { SEEN_THROUGH },
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
    canvas.decals.extend(veils);
}

/// What a decal's level is for a light's glow, which adds all of its picture.
const GLOW: f32 = -1.0;
/// What it is for what is seen through, which lays all of its picture over what is behind.
pub(super) const SEEN_THROUGH: f32 = 1.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_walls_down_from_a_maps_edges_are_left_out_and_its_ground_kept() {
        let face = |points| LandFace {
            points,
            normals: [[0.0, 1.0, 0.0]; 3],
            color: [0.0; 3],
            picture: [NO_PICTURE; 3],
            sea: false,
        };
        let extent = (4608.0, 3072.0);
        let walls = [
            [
                [4608.0, 47.7, 4448.4],
                [4608.0, -500.0, 4448.4],
                [4608.0, -500.0, 4427.2],
            ],
            [[0.0, 20.0, 64.0], [0.0, -500.0, 64.0], [0.0, -500.0, 32.0]],
            [
                [128.0, 30.0, 3072.0],
                [128.0, -500.0, 3072.0],
                [96.0, -500.0, 3072.0],
            ],
        ];
        for points in walls {
            assert!(hangs_from_edge(&face(points), extent), "{points:?}");
        }
        // Ground in a corner touches two edges, but lies along neither.
        let corner = [[0.0, 10.0, 0.0], [32.0, 10.0, 0.0], [0.0, 10.0, 32.0]];
        let inside = [[64.0, 10.0, 64.0], [96.0, 10.0, 64.0], [64.0, 10.0, 96.0]];
        for points in [corner, inside] {
            assert!(!hangs_from_edge(&face(points), extent), "{points:?}");
        }
    }

    #[test]
    fn a_building_shows_damaged_below_half_its_health_and_badly_below_a_quarter() {
        let both = [Some("damaged"), Some("badly")];
        assert_eq!(as_damaged(1.0, &both), None);
        assert_eq!(as_damaged(0.6, &both), None);
        assert_eq!(as_damaged(0.4, &both), Some(&"damaged"));
        assert_eq!(as_damaged(0.1, &both), Some(&"badly"));
        assert_eq!(
            as_damaged(0.1, &[Some("damaged"), None]),
            Some(&"damaged"),
            "the worst there is"
        );
    }

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
    fn a_tornado_is_drawn_from_its_see_through_model_and_without_the_disc_is_not() {
        let triangle = |name: &str| Object {
            name: name.into(),
            transform: IDENTITY,
            animation: None,
            bending: None,
            vertices: [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 900.0, 0.0]]
                .map(|position| xbf::Vertex {
                    position,
                    normal: [0.0, 0.0, -1.0],
                })
                .to_vec(),
            faces: vec![xbf::Face {
                corners: [0, 1, 2],
                texture: Some(0),
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            }],
            children: Vec::new(),
        };
        let funnel = Picture::SeeThrough {
            image: Image {
                width: 1,
                height: 1,
                pixels: vec![[200, 180, 140, 90]],
            },
            layer: 7,
        };
        let mut shape = Shape::default();
        for name in ["?tornado", "?~~2{LEECH}"] {
            gather(
                &triangle(name),
                None,
                &IDENTITY,
                false,
                &[&funnel],
                &mut shape,
            );
        }
        let veils = shape.parts.iter().map(|part| part.veils.len());
        assert_eq!(
            veils.collect::<Vec<_>>(),
            [1, 0],
            "a marker's see-through faces show, but not those that wait on something"
        );
        assert!(shape.parts.iter().all(|part| part.faces.is_empty()));
        let body = Place::turned([0.0; 3], 0.0);
        let posed = Posed {
            body,
            head: body,
            frame: 0,
            slid: 0.0,
            worlds: pose(&shape, 0.0, 0.0, 0),
            set_up: true,
        };
        let mut canvas = Canvas::default();
        draw_posed(&mut canvas, &shape, &posed, [0.0; 3]);
        assert_eq!(canvas.decals.len(), 3);
        assert!(
            canvas
                .decals
                .iter()
                .all(|corner| corner.level == SEEN_THROUGH && corner.picture[2] == 7.0)
        );
        assert!(canvas.decals.iter().any(|corner| corner.y == 900.0));
        // Seen through, it shows only its front: from one side of it, not from the other.
        let seen_from = |x: f32| {
            let mut canvas = Canvas {
                watchers: Some(super::super::models::Watchers {
                    eye: [x, 0.0, 0.0],
                    sun: [0.0, 1.0, 0.0],
                    camera: true,
                    mirrored: false,
                }),
                ..Canvas::default()
            };
            draw_posed(&mut canvas, &shape, &posed, [0.0; 3]);
            canvas.decals.len()
        };
        let mut sides = [seen_from(-1000.0), seen_from(1000.0)];
        sides.sort();
        assert_eq!(sides, [0, 3]);
        // Without the disc's models there is none to draw, and ours is drawn.
        let terrain = Terrain::open_with_cliff(0.0, 2715.0, 271.5);
        let mut combat = crate::weapons::Combat::default();
        while combat.tornados.is_empty() {
            combat.stir_tornados(&terrain, 1.0);
        }
        assert!(!draw_tornado(&mut canvas, &combat.tornados[0], 0.0));
    }

    #[test]
    fn a_soldier_cheers_by_its_win_once_he_stands() {
        let shape = Shape {
            sequences: vec![
                Sequence {
                    name: "Stationary".into(),
                    ranges: vec![(0, 10)],
                },
                Sequence {
                    name: "Win".into(),
                    ranges: vec![(50, 80)],
                },
            ],
            ..Shape::default()
        };
        let mut unit = Unit::for_test(0.0, 0.0, UnitKind::Infantry);
        assert!(frame(&shape, &unit, 0.5) < 10);
        unit.cheering = true;
        assert!((50..80).contains(&frame(&shape, &unit, 0.5)));
    }

    #[test]
    fn a_soldier_dies_by_the_run_for_its_wound_to_its_end_and_lies_so() {
        let sequence = |name: &str, first, last| Sequence {
            name: name.into(),
            ranges: vec![(first, last)],
        };
        let shape = Shape {
            sequences: vec![
                sequence("Shot 1", 100, 130),
                sequence("Shot 2", 140, 160),
                sequence("Burnt 1", 200, 260),
                sequence("Deployed Death 1", 300, 320),
            ],
            ..Shape::default()
        };
        let body = |wound, second, deployed, age| {
            let mut unit = Unit::for_test(0.0, 0.0, UnitKind::Infantry);
            unit.deployed = deployed;
            Body {
                unit,
                wound,
                second,
                age,
            }
        };
        let run = |wound, second, deployed| death_run(&shape, &body(wound, second, deployed, 0.0));
        assert_eq!(run(Wound::Burnt, false, false), Some(&[(200, 260)][..]));
        assert_eq!(
            run(Wound::Burnt, true, false),
            Some(&[(200, 260)][..]),
            "only one"
        );
        assert_eq!(run(Wound::Shot, true, false), Some(&[(140, 160)][..]));
        assert_eq!(run(Wound::BlownUp, false, true), Some(&[(300, 320)][..]));
        assert_eq!(
            run(Wound::Gassed, false, false),
            Some(&[(100, 130)][..]),
            "as if shot without a run of its own"
        );
        let frame = |age| body_frame(&shape, &body(Wound::Burnt, false, false, age));
        assert_eq!(frame(0.0), 200);
        assert_eq!(frame(1.0), 225);
        assert_eq!(frame(3.0), 260, "played to its end");
        assert_eq!(frame(8.0), 260, "and lying so");
    }

    #[test]
    fn the_sea_on_a_maps_ground_is_drawn_as_water_deep_under_it_and_the_land_as_ground() {
        let triangle = |texture: usize, y: f32| Object {
            name: "piece".into(),
            transform: IDENTITY,
            animation: None,
            bending: None,
            vertices: [[0.0, y, 0.0], [32.0, y, 0.0], [0.0, y, 32.0]]
                .map(|position| xbf::Vertex {
                    position,
                    normal: [0.0, 1.0, 0.0],
                })
                .to_vec(),
            faces: vec![xbf::Face {
                corners: [0, 1, 2],
                texture: Some(texture),
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            }],
            children: Vec::new(),
        };
        let seas = [false, true];
        let mut pieces = Vec::new();
        for (texture, y) in [(0, 20.0), (1, -131.0)] {
            gather_land(&triangle(texture, y), &IDENTITY, &[], &seas, &mut pieces);
        }
        let (mut ground, mut water) = (Vec::new(), Vec::new());
        for face in pieces.iter().flat_map(|piece| &piece.faces) {
            lay(face, None, &mut ground, &mut water);
        }
        assert_eq!((ground.len(), water.len()), (3, 3));
        assert!(
            ground
                .iter()
                .all(|corner| corner.y == 20.0 && corner.level == 20.0)
        );
        assert!(
            water
                .iter()
                .all(|corner| corner.y == -131.0 && corner.level == -SEA_DEPTH)
        );
        assert!(water.iter().all(|corner| corner.picture == NO_PICTURE));
    }

    #[test]
    fn a_tornado_surfaces_whirls_and_sinks_by_its_sequences() {
        let sequence = |name: &str, first, last| Sequence {
            name: name.into(),
            ranges: vec![(first, last)],
        };
        let shape = Shape {
            sequences: vec![
                sequence("Move", 150, 300),
                sequence("Sink", 300, 450),
                sequence("Surface", 0, 150),
            ],
            ..Shape::default()
        };
        assert_eq!(tornado_frame(&shape, 0.0, 30.0, 0.0), 0);
        assert_eq!(tornado_frame(&shape, 2.0, 28.0, 0.0), 50);
        assert_eq!(tornado_frame(&shape, 10.0, 20.0, 2.0), 200);
        assert_eq!(tornado_frame(&shape, 28.0, 2.0, 0.0), 400);
        assert_eq!(tornado_frame(&shape, 30.0, 0.0, 0.0), 449);
    }

    #[test]
    fn a_run_plays_the_sounds_in_its_ranges_as_far_into_it_as_they_are() {
        let sounds = [
            (5, "before".to_string()),
            (10, "first".to_string()),
            (14, "second".to_string()),
            (40, "third".to_string()),
        ];
        let heard = sounds_in(&sounds, &[(10, 20), (40, 45)]);
        let at = |frames: usize, name: &str| (frames as f32 / FRAMES_PER_SECOND, name.to_string());
        assert_eq!(heard, [at(0, "first"), at(4, "second"), at(10, "third")]);
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
        spice.lay(&mut decals, &terrain, (101, 100), None);
        assert!(decals.is_empty());
        spice.lay(&mut decals, &terrain, (100, 100), None);
        assert_eq!(decals.len(), 4 * 3);
        let (x, z) = battlefield::grid_to_world(100.5, 100.5);
        let reach = SPICE_SHIFT * 2.0_f32.sqrt() + SPICE_REACH + SPICE_REACH_JITTER;
        for corner in &decals {
            assert!((corner.x - x).hypot(corner.z - z) <= reach);
            assert_eq!(corner.y, SPICE_LIFT);
            assert_eq!(corner.picture[2], 3.0);
            assert_eq!(corner.fog, crate::platform::FOG_CLEAR, "no fog");
        }

        // Under fog, each corner is shaded by the fog where it lies, as the ground is, so a
        // patch fades over the fog's edge rather than showing or not as a whole.
        let mut fog = Fog::new();
        fog.update(std::iter::empty(), 0.0);
        let mut fogged = Vec::new();
        spice.lay(&mut fogged, &terrain, (100, 100), Some(&fog));
        for corner in &fogged {
            assert_eq!(corner.seen, 1.0);
            assert_eq!(corner.fog, fog.edges_at(corner.x, corner.z));
        }
    }

    #[test]
    fn a_deployed_kobras_cannon_on_the_disc_swings_round_the_way_it_aims() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(index) = archive::from_cabinet(&disc.join(CABINET), INDEX) else {
            return;
        };
        let archive = Archive::open(&index, &disc.join(DATA)).unwrap();
        let shape = read_shape(&archive, "Units\\OR_Kobra_H0.XBF", &mut HashMap::new()).unwrap();
        assert!(shape.parts.iter().any(|part| part.swivel));
        let mut kobra = Unit::for_test(1000.0, 1000.0, UnitKind::Vehicle);
        kobra.role = Role::OrKobra;
        crate::units::switch_deployed(&mut kobra);
        kobra.popped = 1.0;
        let swivel = shape.parts.iter().position(|part| part.swivel).unwrap();
        for turn in [0.0, 1.5, -2.5] {
            kobra.gun_turn = turn;
            // Seen from what it swings round on.
            let worlds = pose(&shape, kobra.tube_pitch, turn, frame(&shape, &kobra, 0.0));
            let at = worlds[swivel][3];
            let [px, _, pz] = places(&shape, &kobra)
                .0
                .point(to_place([at[0], at[1], at[2]]));
            let (x, _, z) = ends_of(&shape, &kobra)[1];
            let way = (z - pz).atan2(x - px);
            let off = crate::units::turn_between(way, kobra.heading + turn);
            assert!(off.abs() < 0.2, "swung {turn}: off by {off}");
        }
    }

    #[test]
    fn a_wall_on_the_disc_reaches_out_towards_each_wall_it_joins() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(index) = archive::from_cabinet(&disc.join(CABINET), INDEX) else {
            return;
        };
        let archive = Archive::open(&index, &disc.join(DATA)).unwrap();
        for (house, pieces) in WALLS {
            for name in pieces {
                assert!(
                    read_shape(&archive, name, &mut HashMap::new()).is_some(),
                    "{name}"
                );
            }
            let shape = read_shape(&archive, pieces[1], &mut HashMap::new()).unwrap();
            let mut wall = Unit::for_test(1000.0, 1000.0, UnitKind::Building);
            (wall.role, wall.house) = (Role::Wall, house);
            // As made, an end joins towards its +x, where it reaches the square's edge.
            for (side, way) in [
                (0, (16.0, 0.0)),
                (1, (0.0, 16.0)),
                (2, (-16.0, 0.0)),
                (3, (0.0, -16.0)),
            ] {
                wall.wall_joins = std::array::from_fn(|i| i == side);
                let [x, _, z] = places(&shape, &wall).0.point(to_place([16.0, 0.0, 0.0]));
                let off = ((x - wall.x - way.0).abs(), (z - wall.z - way.1).abs());
                assert!(
                    off.0 < 0.01 && off.1 < 0.01,
                    "{house:?} side {side}: {off:?}"
                );
            }
        }
    }

    #[test]
    fn a_windtrap_on_the_disc_blows_wind_that_flows_across_it() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(index) = archive::from_cabinet(&disc.join(CABINET), INDEX) else {
            return;
        };
        let archive = Archive::open(&index, &disc.join(DATA)).unwrap();
        for name in [
            "Buildings\\AT_Windtrap_H0.XbF",
            "Buildings\\HK_Windtrap_H0.XBF",
            "Buildings\\or_windtrap_H0.XbF",
        ] {
            let shape = read_shape(&archive, name, &mut HashMap::new()).unwrap();
            let wind: Vec<&Part> = shape
                .parts
                .iter()
                .filter(|part| part.veils.iter().any(|veil| veil.glows) && part.slide[0] > 0.0)
                .collect();
            assert!(wind.len() >= 2, "{name}: its two streams of wind");
        }
    }

    #[test]
    fn each_kind_of_scenery_has_its_model_on_the_disc() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(original) = read_shapes(disc) else {
            return;
        };
        for &(rules_name, name) in SCENERY {
            assert!(original.shapes.contains_key(name), "{rules_name}: {name}");
        }
    }

    #[test]
    fn each_extra_kind_has_its_model_on_the_disc_and_is_drawn_with_it() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(original) = read_shapes(disc) else {
            return;
        };
        for &(rules_name, name) in EXTRAS {
            assert!(original.shapes.contains_key(name), "{rules_name}: {name}");
            let (role, _) = Role::named(rules_name).unwrap();
            let mut unit = Unit::for_test(0.0, 0.0, role.kind());
            unit.role = role;
            assert_eq!(model_name(&unit), Some(name), "{rules_name}");
        }
    }

    #[test]
    fn each_kind_of_crate_has_its_model_on_the_disc() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(original) = read_shapes(disc) else {
            return;
        };
        let _ = ORIGINAL.set(original);
        for (kind, _) in CRATES {
            let mut canvas = Canvas::default();
            assert!(draw_crate(&mut canvas, kind, [0.0; 3], 0.0), "{kind:?}");
            assert!(!canvas.objects.is_empty(), "{kind:?}");
        }
    }

    #[test]
    fn a_mortar_on_the_disc_shows_the_mortar_it_sets_up_only_once_dug_in() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(original) = read_shapes(disc) else {
            return;
        };
        let _ = ORIGINAL.set(original);
        let mut mortar = Unit::for_test(1000.0, 1000.0, UnitKind::Infantry);
        mortar.role = Role::OrMortar;
        let set_up = |unit: &Unit| {
            shape_of(unit)
                .unwrap()
                .parts
                .iter()
                .filter(|p| p.set_up)
                .count()
        };
        assert_eq!(set_up(&mortar), 1, "its Mortorgun01");
        let mut kindjal = mortar;
        kindjal.role = Role::AtKindjal;
        assert_eq!(set_up(&kindjal), 0, "a Kindjal carries the gun it sets up");
        let drawn = |unit: &Unit| {
            let mut canvas = Canvas::default();
            draw(&mut canvas, unit, [1.0; 3], 0.0);
            canvas.objects.len()
        };
        let walking = drawn(&mortar);
        mortar.deployed = true;
        assert!(drawn(&mortar) > walking, "set up, it stands beside him");
        let mut turret = Unit::for_test(1000.0, 1000.0, UnitKind::Tower);
        turret.role = Role::AtRocketTurret;
        let standing = drawn(&turret);
        turret.deployed = true;
        assert_eq!(
            drawn(&turret),
            standing,
            "a rocket turret's head is always on it"
        );
    }

    #[test]
    fn a_windtraps_front_and_back_panels_on_the_disc_show_unlit_and_hide_what_is_behind() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(index) = archive::from_cabinet(&disc.join(CABINET), INDEX) else {
            return;
        };
        let archive = Archive::open(&index, &disc.join(DATA)).unwrap();
        for name in [
            "Buildings\\AT_Windtrap_H0.XbF",
            "Buildings\\HK_Windtrap_H0.XBF",
            "Buildings\\or_windtrap_H0.XbF",
        ] {
            let mut pictures = HashMap::new();
            let shape = read_shape(&archive, name, &mut pictures).unwrap();
            let panels: Vec<&Part> = shape
                .parts
                .iter()
                .filter(|part| part.beam && !part.veils.is_empty())
                .collect();
            assert!(panels.len() >= 2, "{name}: {} panels", panels.len());
            assert!(
                panels
                    .iter()
                    .all(|part| part.veils.iter().all(|veil| !veil.glows)),
                "{name}: they cover, not add"
            );
        }
    }

    #[test]
    fn a_landing_pads_deck_on_the_disc_lies_where_machines_sit_on_it() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(index) = archive::from_cabinet(&disc.join(CABINET), INDEX) else {
            return;
        };
        let archive = Archive::open(&index, &disc.join(DATA)).unwrap();
        for name in [
            "Buildings\\at_helipad_H0.XbF",
            "Buildings\\hk_helipad_H0.XbF",
        ] {
            let model = xbf::parse(&archive.read(name).unwrap()).unwrap();
            // The heights of its level faces, the deck's the most of them near the top.
            fn level(object: &Object, above: &[[f32; 4]; 4], heights: &mut Vec<f32>) {
                let world = times(&object.transform, above);
                for face in &object.faces {
                    let y = face
                        .corners
                        .map(|c| to_place(point(&world, object.vertices[c].position))[1]);
                    if (y[0] - y[1]).abs() < 0.5 && (y[0] - y[2]).abs() < 0.5 {
                        heights.push(y[0]);
                    }
                }
                for child in &object.children {
                    level(child, &world, heights);
                }
            }
            let mut heights = Vec::new();
            for object in &model.objects {
                level(object, &IDENTITY, &mut heights);
            }
            let deck = heights
                .iter()
                .filter(|&&y| (y - crate::units::pad_top()).abs() < 1.0);
            assert!(deck.count() >= 4, "{name}: {heights:?}");
        }
    }

    #[test]
    fn a_refinerys_side_pads_slide_out_onto_where_its_harvesters_dock_only_once_upgraded() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(original) = read_shapes(disc) else {
            return;
        };
        let _ = ORIGINAL.set(original);
        use crate::units::Facing;
        for house in [Faction::Atreides, Faction::Harkonnen, Faction::Ordos] {
            for facing in [Facing::LeftBottom, Facing::RightTop, Facing::RightBottom] {
                let mut refinery = Unit::for_test(1000.0, 1000.0, UnitKind::Building);
                refinery.role = Role::Refinery;
                refinery.house = house;
                refinery.facing = facing;
                let shape = shape_of(&refinery).unwrap();
                // The middle of each sliding pad on the ground, by its number.
                let pads = |refinery: &Unit| {
                    let (body, _) = places(shape, refinery);
                    let worlds = pose_by(shape, 0.0, 0.0, |part| {
                        part.pad.map_or(0, |pad| pad_frame(shape, refinery, pad))
                    });
                    let mut middles = [(0.0, 0.0); 2];
                    for (part, world) in shape.parts.iter().zip(&worlds) {
                        let Some(pad) = part.pad else { continue };
                        let (mut low, mut high) = ([f32::MAX; 2], [f32::MIN; 2]);
                        for &p in part.faces.iter().flat_map(|f| &f.points) {
                            let [x, _, z] = body.point(to_place(point(world, p)));
                            low = [low[0].min(x), low[1].min(z)];
                            high = [high[0].max(x), high[1].max(z)];
                        }
                        middles[usize::from(pad) - 1] =
                            ((low[0] + high[0]) / 2.0, (low[1] + high[1]) / 2.0);
                    }
                    middles
                };
                let docks = refinery.refinery_pads();
                let near = |(x, z): (f32, f32), (dx, dz): (f32, f32)| (x - dx).hypot(z - dz) < 3.0;
                let tucked = pads(&refinery);
                assert!(
                    !near(tucked[0], docks[2]) && !near(tucked[1], docks[0]),
                    "{house:?} {facing:?}: tucked in before any upgrade"
                );
                refinery.upgrades = 1;
                refinery.popped = 0.5;
                let sliding = pads(&refinery);
                assert!(!near(sliding[0], docks[2]) && sliding[0] != tucked[0]);
                refinery.popped = 1.0;
                let first = pads(&refinery);
                assert!(
                    near(first[0], docks[2]) && first[1] == tucked[1],
                    "{house:?} {facing:?}: the first out on its right, {first:?} {docks:?}"
                );
                refinery.upgrades = 2;
                let both = pads(&refinery);
                assert!(
                    near(both[0], docks[2]) && near(both[1], docks[0]),
                    "{house:?} {facing:?}: and the second on its left"
                );
            }
        }
    }

    #[test]
    fn a_searchlight_on_the_disc_lights_the_ground_where_its_beam_sweeps() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(original) = read_shapes(disc) else {
            return;
        };
        let _ = ORIGINAL.set(original);
        let mut light = Unit::for_test(1000.0, 1000.0, UnitKind::Building);
        light.role = Role::Prop(Prop::Spotlight);
        let (first, later) = (
            searchlight_spots(&light, 0.0),
            searchlight_spots(&light, 2.0),
        );
        assert_eq!((first.len(), later.len()), (1, 1));
        let ([x, y, z], spread) = first[0];
        assert!(
            (x - light.x).hypot(z - light.z) > 100.0,
            "out where it points"
        );
        assert!(y < 20.0, "on the ground");
        assert!(spread > 100.0, "as wide as its beam's end: {spread}");
        assert!(
            (later[0].0[0] - x).hypot(later[0].0[2] - z) > 30.0,
            "sweeping"
        );
        // It sweeps on evenly, never leaping, between frames as well as at them.
        let steps = (0..200).map(|i| searchlight_spots(&light, i as f32 * 0.01)[0].0);
        let steps: Vec<[f32; 3]> = steps.collect();
        for pair in steps.windows(2) {
            let step = (pair[1][0] - pair[0][0]).hypot(pair[1][2] - pair[0][2]);
            assert!(step < 2.0, "a step of {step}");
        }
        let mut windtrap = Unit::for_test(1000.0, 1000.0, UnitKind::Building);
        windtrap.role = Role::Windtrap;
        assert!(
            searchlight_spots(&windtrap, 0.0).is_empty(),
            "lights only itself"
        );
    }

    #[test]
    fn the_atreides_factorys_and_a_starports_lights_glow_where_their_marks_are_not_as_boxes() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(original) = read_shapes(disc) else {
            return;
        };
        let _ = ORIGINAL.set(original);
        let mut factory = Unit::for_test(1000.0, 1000.0, UnitKind::Building);
        (factory.role, factory.house) = (Role::Factory, Faction::Atreides);
        let mut starport = factory;
        starport.role = Role::Starport;
        for (unit, lights) in [(factory, 8), (starport, 8)] {
            let glints = glints(&unit, 0.0);
            assert_eq!(glints.len(), lights, "{:?}", unit.role);
            for (x, _, z) in glints {
                assert!((x - unit.x).hypot(z - unit.z) < 200.0, "on it");
            }
            let shape = shape_of(&unit).unwrap();
            let marks = shape.parts.iter().filter(|part| part.glint.is_some());
            assert!(
                marks
                    .into_iter()
                    .all(|part| part.faces.is_empty() && part.veils.is_empty())
            );
        }
        let mut windtrap = factory;
        windtrap.role = Role::Windtrap;
        let shape = shape_of(&windtrap).unwrap();
        assert!(
            shape
                .parts
                .iter()
                .any(|part| part.veils.iter().any(|veil| !veil.glows))
        );
    }

    #[test]
    fn a_mongooses_searchlight_on_the_disc_lights_the_ground_ahead_of_it() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(original) = read_shapes(disc) else {
            return;
        };
        let _ = ORIGINAL.set(original);
        let mut mongoose = Unit::for_test(1000.0, 1000.0, UnitKind::Vehicle);
        mongoose.role = Role::AtMongoose;
        for heading in [0.0, FRAC_PI_2] {
            mongoose.heading = heading;
            let spots = searchlight_spots(&mongoose, 0.0);
            assert_eq!(spots.len(), 1);
            let ([x, _, z], _) = spots[0];
            let way = (z - mongoose.z).atan2(x - mongoose.x);
            assert!(
                crate::units::turn_between(way, heading).abs() < 0.6,
                "ahead: {way}"
            );
            assert!((x - mongoose.x).hypot(z - mongoose.z) > 30.0, "out from it");
        }
    }

    #[test]
    fn the_discs_pointer_hangs_right_of_and_below_its_tip_as_on_the_screen() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(original) = read_shapes(disc) else {
            return;
        };
        let _ = ORIGINAL.set(original);
        let mut vertices = Vec::new();
        assert!(draw_cursor(
            &mut vertices,
            CursorLook::Arrow,
            (0.0, 0.0),
            10.0,
            0.0
        ));
        let mean =
            |at: fn(&Vertex) -> f32| vertices.iter().map(at).sum::<f32>() / vertices.len() as f32;
        let (x, y) = (mean(|v| v.x), mean(|v| v.y));
        assert!(x > 0.0 && y > 0.0, "it lies at ({x}, {y}) from its tip");
    }

    #[test]
    fn the_discs_hourglass_turns_over_where_the_pointer_is() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(original) = read_shapes(disc) else {
            return;
        };
        let _ = ORIGINAL.set(original);
        let drawn = |time: f32| {
            let mut vertices = Vec::new();
            assert!(draw_cursor(
                &mut vertices,
                CursorLook::Wait,
                (500.0, 400.0),
                3.0,
                time
            ));
            vertices
        };
        // It stands one way up for its first 40 frames, and the other from then on to 90.
        let (first, later) = (drawn(0.0), drawn(2.0));
        assert!(!first.is_empty());
        for v in &first {
            assert!(
                (v.x - 500.0).hypot(v.y - 400.0) < 3.0 * 20.0,
                "about the pointer"
            );
        }
        let xs = |vertices: &[Vertex]| vertices.iter().map(|v| v.x).collect::<Vec<_>>();
        let ys = |vertices: &[Vertex]| vertices.iter().map(|v| v.y).collect::<Vec<_>>();
        assert_ne!(
            (xs(&first), ys(&first)),
            (xs(&later), ys(&later)),
            "it turns over"
        );
    }

    #[test]
    fn the_discs_lettering_has_every_letter_of_its_list() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(listed) = read_interface(disc, FONT_LIST) else {
            return;
        };
        let index = archive::from_cabinet(&disc.join(CABINET), INDEX).unwrap();
        let archive = Archive::open(&index, &disc.join(DATA)).unwrap();
        let image = xbf::tga(&archive.read(&format!("{TEXTURES}{FONT_PICTURE}")).unwrap()).unwrap();
        let (mut pictures, mut layers) = (Vec::new(), 0);
        let lettering = lettering::read(&image, &listed, &mut pictures, &mut layers).unwrap();
        for ch in ['A', 'g', '!', '0', '\u{df}', '\u{a9}'] {
            assert!(lettering.letter(ch).is_some(), "{ch}");
        }
        assert!(lettering.capital > 10.0);
        let g = lettering.letter('g').unwrap();
        assert!(g.below > 0.0, "a g reaches below the line");
        assert_eq!(pictures.len(), layers * PICTURE_SIDE * PICTURE_SIDE * 4);
    }
}
