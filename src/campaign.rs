//! The campaign between its battles, as the original plays it. Arrakis is cut into
//! territories the three houses hold. Each turn the player attacks one next to theirs, and
//! then the other houses attack too, the player being asked whether to defend what they
//! attack of theirs. The campaign goes through phases, each moving on once enough battles are
//! won, and each ending in a mission off the map: a Heighliner, a homeworld to defend, the
//! Harkonnen's civil war with its side to choose, an enemy's homeworld, and the last battle.
//! The tech level rises with them, bringing new units.
//!
//! What it is made of is on the disc: which territory touches which, in the campaign's data;
//! who holds each at the start and its shape, in the original's campaign screen
//! `frontend\campaign.xbf`; the phases' and tech levels' rules in `PhaseRules.txt`; the houses'
//! jump points in `Forced Missions.txt`; and the new units of each tech level in
//! `NEWUNITS.INI`. Which phase means which mission, which the data does not say, is here as
//! the game's own messages about the phases tell it.

use crate::archive;
use crate::battlefield::next_random;
use crate::game::Faction;
use crate::maps;
use crate::mission::{self, Token};
use crate::xbf;
use std::path::Path;
use std::sync::OnceLock;

const CABINET: &str = "INSTALL/Game1.cab";
const CAMPAIGN_INDEX: &str = "\\data\\CAMPAIGN0001.RFH";
const CAMPAIGN_DATA: &str = "\\data\\CAMPAIGN0001.RFD";
const UI_INDEX: &str = "\\data\\UI0001.RFH";
const UI_DATA: &str = "\\data\\UI0001.RFD";
const TEXT_INDEX: &str = "\\data\\STRINGS0001.RFH";
const TEXT_DATA: &str = "\\data\\STRINGS0001.RFD";
const CONNECTIONS: &str = "arrakis connections.txt";
const PHASE_RULES: &str = "PhaseRules.txt";
const FORCED_MISSIONS: &str = "Forced Missions.txt";
/// Every mission of the campaign, by the name of its text, kept with their scripts.
const MISSION_LIST: &str = "missionlist.ini";
const SCENE: &str = "FRONTEND\\CAMPAIGN.XBF";
/// The missions' scripts, and the rules whose lists of kinds the scripts' kinds count in.
const MISSIONS_INDEX: &str = "\\data\\MISSIONS0001.RFH";
const MISSIONS_DATA: &str = "\\data\\MISSIONS0001.RFD";
const MODEL_INDEX: &str = "\\data\\MODEL0001.RFH";
const MODEL_DATA: &str = "\\data\\MODEL0001.RFD";
const RULES: &str = "Rules.txt";
const KIND_LISTS: [&str; 2] = ["[BuildingTypes]", "[UnitTypes]"];
/// The script of each house's first battle.
const START_SCRIPTS: [(Faction, &str); 3] = [
    (Faction::Atreides, "ATStart.tok"),
    (Faction::Harkonnen, "HKStart.tok"),
    (Faction::Ordos, "ORStart.tok"),
];
const NEW_UNITS: &str = "NEWUNITS.INI";
/// The game's text: the newer table first, whose names win, then the older one; and the
/// table of what is spoken, which says what each unit is.
const TEXTS: [&str; 2] = ["E_Output_Text_Pickup.txt", "Text strings.txt"];
const SPOKEN: &str = "UISPOKEN.TXT";
/// The part of the campaign screen that is the map of Arrakis, and what the names of a
/// territory's shape and of its rim start with, followed by its number.
const MAP: &str = "~~1map#";
const SHAPE: char = 'F';
const RIM: char = 'E';
/// The pictures a rim is painted with, by the house that holds the territory.
const RIM_PICTURES: [(&str, Faction); 3] = [
    ("%atblue.tga", Faction::Atreides),
    ("%hkred.tga", Faction::Harkonnen),
    ("%orgreen.tga", Faction::Ordos),
];
/// How far the original's camera is from a screen 600 tall: its ring frame stands 300 above
/// the middle at a depth of 525.
const FOCAL: f32 = 525.0;
/// How many of the territories are on Arrakis; the rest are the houses' homeworlds.
pub const ARRAKIS: usize = 33;
/// The houses' jump points in `Forced Missions.txt`, counted from 0.
const JUMP_POINTS: [(&str, Faction); 3] = [
    ("ATJumpPoint", Faction::Atreides),
    ("HKJumpPoint", Faction::Harkonnen),
    ("ORJumpPoint", Faction::Ordos),
];

/// The phases the original gives a meaning: the first battle, the three on
/// Arrakis, and the missions they jump to.
const START_PHASE: u32 = 0;
const FIRST_PHASE: u32 = 1;
const LAST_ARRAKIS_PHASE: u32 = 3;
const HEIGHLINER_PHASE: u32 = 10;
const HOME_DEFENCE_PHASE: u32 = 11;
const HOMEWORLD_PHASE: u32 = 12;
const FINAL_PHASE: u32 = 13;
/// The Harkonnen's two missions in place of a homeworld defence, one for each side of their
/// civil war.
const COPEC_PHASE: u32 = 14;
const GUNSENG_PHASE: u32 = 15;

/// The chance that another house attacks at all in its turn, and that one attacking a third
/// house wins: the original works these out of its armies' reserves, which are not kept here.
const ATTACK_CHANCE: f32 = 0.5;
const AI_WIN_CHANCE: f32 = 0.5;

/// One of the campaign's territories.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Territory {
    pub name: String,
    /// The territories it touches, by their places in the list.
    pub neighbours: Vec<usize>,
    /// Who holds it as a campaign starts.
    pub first_owner: Option<Faction>,
    /// Its shape on the original's map, on the original's screen 600 tall measured from its
    /// middle, up being up: the triangles that fill it, and those of its rim.
    pub fill: Vec<[(f32, f32); 3]>,
    pub rim: Vec<[(f32, f32); 3]>,
    /// Its middle on that map; a homeworld, off the map, has none.
    pub centre: Option<(f32, f32)>,
}

/// A unit the new units screen shows once its tech level is reached.
#[derive(Clone, Debug, PartialEq)]
pub struct NewUnit {
    pub level: u32,
    /// Whose it is; nobody's is everybody's.
    pub house: Option<Faction>,
    pub name: String,
    pub about: String,
}

/// What a phase asks for before it moves on.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Phase {
    battles: u32,
    captured: u32,
    most_battles: Option<u32>,
    /// Whether it moves on only once an enemy's jump point is taken.
    jump_point: bool,
    jump_to: Option<u32>,
    /// After how many battles in a row without a territory gained the player is warned, and
    /// loses.
    warning: Option<u32>,
    lose: Option<u32>,
}

/// What brings the next tech level.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Rule {
    Phase(u32),
    Captured(u32),
}

/// The campaign as the disc has it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Data {
    /// The territories of Arrakis, then the homeworlds.
    pub territories: Vec<Territory>,
    jump_points: Vec<(Faction, usize)>,
    phases: Vec<(u32, Phase)>,
    tech: Vec<Rule>,
    new_units: Vec<NewUnit>,
    /// The script of each house's first battle, as lines of tokens.
    start_scripts: Vec<(Faction, Vec<Vec<Token>>)>,
    /// The names of the kinds the scripts count, in the rules' order.
    kinds: Vec<String>,
    /// The scripts of the battles for territories, by name without `.tok`, such as
    /// `ATP1D7FR`: the house, the phase, an attack (`M`) or a defence (`D`), the territory's
    /// number, then whom the battle is with.
    mission_scripts: Vec<(String, Vec<Vec<Token>>)>,
}

impl Data {
    /// The corners of the map of Arrakis, as (lowest left, highest right): around every
    /// territory's shape, or its middle where it has none.
    pub fn bounds(&self) -> Option<((f32, f32), (f32, f32))> {
        let points = self.territories.iter().flat_map(|territory| {
            let shape = territory.fill.iter().flatten().copied();
            shape.chain(territory.centre)
        });
        points.fold(None, |around, (x, y)| {
            let ((left, bottom), (right, top)) = around.unwrap_or(((x, y), (x, y)));
            Some(((left.min(x), bottom.min(y)), (right.max(x), top.max(y))))
        })
    }
}

static DATA: OnceLock<Data> = OnceLock::new();

/// Reads the campaign off the disc mounted at `disc`. Without it there is none.
pub fn load(disc: &Path) {
    if let Some(data) = read(disc) {
        let _ = DATA.set(data);
    }
}

/// The campaign the disc has, once read. Tests have one of their own.
pub fn data() -> Option<&'static Data> {
    #[cfg(test)]
    return Some(DATA.get_or_init(tests::data));
    #[cfg(not(test))]
    DATA.get()
}

fn read(disc: &Path) -> Option<Data> {
    let cabinet = std::fs::read(disc.join(CABINET)).ok()?;
    let from = |index: &str, data: &str, name: &str| {
        let index = archive::in_cabinet(&cabinet, index)?;
        let data = archive::in_cabinet(&cabinet, data)?;
        archive::read_in(&index, &data, name)
    };
    let campaign = |name: &str| {
        from(CAMPAIGN_INDEX, CAMPAIGN_DATA, name).map(|b| String::from_utf8_lossy(&b).into_owned())
    };
    let texts: Vec<String> = TEXTS
        .iter()
        .filter_map(|name| from(TEXT_INDEX, TEXT_DATA, name))
        .map(|bytes| maps::utf16(&bytes))
        .collect();
    let named = |id: &str| texts.iter().find_map(|text| maps::name_of(text, id));
    let spoken = from(TEXT_INDEX, TEXT_DATA, SPOKEN)
        .map(|bytes| maps::utf16(&bytes))
        .unwrap_or_default();
    let mut territories = territories(&campaign(CONNECTIONS)?, |n| named(&format!("#T{n}")));
    let scene = xbf::parse(&from(UI_INDEX, UI_DATA, SCENE)?)?;
    shape_territories(&scene, &mut territories);
    let jump_points = jump_points(&campaign(FORCED_MISSIONS)?);
    own_homeworlds(&mut territories, &jump_points);
    let (phases, tech) = rules(&campaign(PHASE_RULES)?);
    let new_units = from(UI_INDEX, UI_DATA, NEW_UNITS)
        .map(|bytes| {
            new_units(&String::from_utf8_lossy(&bytes), |id| {
                (named(id), maps::name_of(&spoken, id))
            })
        })
        .unwrap_or_default();
    let start_scripts = START_SCRIPTS
        .iter()
        .filter_map(|&(house, name)| {
            let script = from(MISSIONS_INDEX, MISSIONS_DATA, name)?;
            Some((house, mission::read(&script)?))
        })
        .collect();
    // The missions the campaign lists, each with its script; their win and fail texts have
    // none.
    let listed = from(MISSIONS_INDEX, MISSIONS_DATA, MISSION_LIST)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    let mission_scripts = listed
        .lines()
        .filter_map(|line| line.trim().strip_suffix(".txt"))
        .filter_map(|name| {
            let script = from(MISSIONS_INDEX, MISSIONS_DATA, &format!("{name}.tok"))?;
            Some((name.to_string(), mission::read(&script)?))
        })
        .collect();
    let kinds = from(MODEL_INDEX, MODEL_DATA, RULES)
        .map(|rules| kinds(&String::from_utf8_lossy(&rules)))
        .unwrap_or_default();
    Some(Data {
        territories,
        jump_points,
        phases,
        tech,
        new_units,
        start_scripts,
        kinds,
        mission_scripts,
    })
}

/// The script of a house's first battle, if the disc has it.
pub fn start_script(house: Faction) -> Option<&'static [Vec<Token>]> {
    let data = data()?;
    let (_, script) = data.start_scripts.iter().find(|(of, _)| *of == house)?;
    Some(script)
}

/// The script of a house's battle for a territory in a phase, if the disc has one: its
/// defence of it, or its attack on it.
pub fn mission_script(
    house: Faction,
    phase: u32,
    defence: bool,
    territory: usize,
) -> Option<&'static [Vec<Token>]> {
    let prefix = format!(
        "{}P{phase}{}{}",
        crate::units::house_letters(house),
        if defence { 'D' } else { 'M' },
        territory + 1
    );
    data()?
        .mission_scripts
        .iter()
        .find(|(name, _)| name.strip_prefix(&prefix).is_some_and(names_whom))
        .map(|(_, script)| script.as_slice())
}

/// The scripts of the battles past Arrakis' territories, each a house's, by their names on
/// the disc: the Heighliner, the homeworlds' defences, the Harkonnen civil war's sides (its
/// defence on the homeworld for Copec, its attack for Gunseng), the assaults on the others'
/// homeworlds and the last battle.
const BATTLE_SCRIPTS: [(Faction, Mission, &str); 16] = {
    use Faction::{Atreides, Harkonnen, Ordos};
    [
        (Atreides, Mission::Heighliner, "Atreides Heighliner Mission"),
        (Harkonnen, Mission::Heighliner, "HHK Heighliner Mission"),
        (Ordos, Mission::Heighliner, "Ordos Heighliner Mission"),
        (Atreides, Mission::HomeDefence, "DAT Save The Duke"),
        (Ordos, Mission::HomeDefence, "Ordos Homeworld Defense"),
        (
            Harkonnen,
            Mission::CivilWar(Side::Copec),
            "HHK Civil War Defence Mission",
        ),
        (
            Harkonnen,
            Mission::CivilWar(Side::Gunseng),
            "HHK Civil War Attack Mission",
        ),
        (
            Atreides,
            Mission::Homeworld(Harkonnen),
            "Harkonnen homeworld assault_AT",
        ),
        (
            Ordos,
            Mission::Homeworld(Harkonnen),
            "Harkonnen homeworld assault_OR",
        ),
        (
            Atreides,
            Mission::Homeworld(Ordos),
            "Ordos homeworld assault _Atreides",
        ),
        (
            Harkonnen,
            Mission::Homeworld(Ordos),
            "Ordos homeworld assault",
        ),
        (
            Harkonnen,
            Mission::Homeworld(Atreides),
            "T36 Atreides Homeworld Assault",
        ),
        (
            Ordos,
            Mission::Homeworld(Atreides),
            "T36 Atreides Homeworld Assault_OR",
        ),
        (Atreides, Mission::Final, "ATENDMission"),
        (Harkonnen, Mission::Final, "HKENDMission"),
        (Ordos, Mission::Final, "ORENDMission"),
    ]
};

/// The script of a house's battle past the territories, if the disc has it.
/// The original's tutorial: its map and its script.
pub const TUTORIAL_MAP: &str = "#x1";
const TUTORIAL_SCRIPT: &str = "ATTutorial";

/// The tutorial's script, if the disc has it.
pub fn tutorial_script() -> Option<&'static [Vec<Token>]> {
    data()?
        .mission_scripts
        .iter()
        .find(|(listed, _)| listed == TUTORIAL_SCRIPT)
        .map(|(_, script)| script.as_slice())
}

pub fn battle_script(house: Faction, mission: Mission) -> Option<&'static [Vec<Token>]> {
    let &(.., name) = BATTLE_SCRIPTS
        .iter()
        .find(|&&(of, battle, _)| of == house && battle == mission)?;
    data()?
        .mission_scripts
        .iter()
        .find(|(listed, _)| listed == name)
        .map(|(_, script)| script.as_slice())
}

/// The phase a battle past the territories is fought in.
fn phase_of(mission: Mission) -> u32 {
    match mission {
        Mission::Heighliner => HEIGHLINER_PHASE,
        Mission::HomeDefence => HOME_DEFENCE_PHASE,
        Mission::Homeworld(_) => HOMEWORLD_PHASE,
        Mission::Final => FINAL_PHASE,
        Mission::CivilWar(Side::Copec) => COPEC_PHASE,
        Mission::CivilWar(Side::Gunseng) => GUNSENG_PHASE,
        Mission::Start => START_PHASE,
        Mission::Attack(_) | Mission::Defend(..) => FIRST_PHASE,
    }
}

/// Whether what ends a territory's script's name says whom the battle is with, in capitals
/// (`FR`, `SA`): territory 1 is not 11, and its `…Fail` and `…Win` scripts are others'.
fn names_whom(rest: &str) -> bool {
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_uppercase())
}

/// The house, the phase and the battle of a script, by its name: `ATP1M7FR` is the
/// Atreides' attack on territory 7 in phase 1, one of a defence fought against who held the
/// territory at first; `Atreides Heighliner Mission` is theirs on the Heighliner, as
/// `BATTLE_SCRIPTS` has those past the territories, and `ATStart` their first battle.
pub fn named_mission(name: &str) -> Option<(Faction, u32, Mission)> {
    // Their names may have spaces, which may be left out on a command line.
    let squeezed = |name: &str| name.replace(' ', "").to_lowercase();
    if let Some(&(house, mission, _)) = BATTLE_SCRIPTS
        .iter()
        .find(|(.., script)| squeezed(script) == squeezed(name))
    {
        return Some((house, phase_of(mission), mission));
    }
    if let Some(&(house, _)) = START_SCRIPTS.iter().find(|(_, script)| {
        script
            .strip_suffix(".tok")
            .is_some_and(|start| start == name)
    }) {
        return Some((house, START_PHASE, Mission::Start));
    }
    let house = [Faction::Atreides, Faction::Harkonnen, Faction::Ordos]
        .into_iter()
        .find(|&house| name.starts_with(crate::units::house_letters(house)))?;
    let digits = |s: &str| s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let rest = name[2..].strip_prefix('P')?;
    let (phase, rest) = rest.split_at(digits(rest));
    let phase = phase.parse().ok()?;
    let (defence, rest) = if let Some(rest) = rest.strip_prefix('D') {
        (true, rest)
    } else {
        (false, rest.strip_prefix('M')?)
    };
    let (territory, rest) = rest.split_at(digits(rest));
    let at = territory
        .parse::<usize>()
        .ok()?
        .checked_sub(1)
        .filter(|&at| at < ARRAKIS)?;
    if !names_whom(rest) {
        return None;
    }
    if !defence {
        return Some((house, phase, Mission::Attack(at)));
    }
    let by = data()
        .and_then(|data| data.territories.get(at)?.first_owner)
        .filter(|&owner| owner != house)
        .unwrap_or(house.next());
    Some((house, phase, Mission::Defend(at, by)))
}

/// The names of the battles' scripts on the disc that `named_mission` knows: the houses'
/// first battles', the territories' and those past them.
pub fn mission_names() -> Vec<&'static str> {
    data().map_or_else(Vec::new, |data| {
        data.mission_scripts
            .iter()
            .map(|(name, _)| name.as_str())
            .filter(|name| named_mission(name).is_some())
            .collect()
    })
}

/// What the rules call the kind a script counts as this, if they have it.
pub fn kind_name(kind: u16) -> Option<&'static str> {
    data()?.kinds.get(usize::from(kind)).map(String::as_str)
}

/// The kinds of the rules' lists of buildings and units, in the order of the file: the lists
/// come several times, each adding to its own.
fn kinds(rules: &str) -> Vec<String> {
    let mut listing = false;
    let mut kinds = Vec::new();
    for line in rules.lines() {
        let line = line.split("//").next().unwrap_or("").trim();
        if line.starts_with('[') {
            listing = KIND_LISTS.contains(&line);
        } else if listing && let Some(kind) = line.split_whitespace().next() {
            kinds.push(kind.to_string());
        }
    }
    kinds
}

/// The territories out of their connections, a line for each listing the numbers, from 1, of
/// those it touches; `name` what the game's text calls the one of a number.
fn territories(connections: &str, name: impl Fn(usize) -> Option<String>) -> Vec<Territory> {
    connections
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .enumerate()
        .map(|(at, line)| Territory {
            name: name(at + 1).unwrap_or_else(|| format!("Territory {}", at + 1)),
            neighbours: line
                .split(',')
                .filter_map(|n| n.trim().parse::<usize>().ok())
                .filter_map(|n| n.checked_sub(1))
                .collect(),
            ..Territory::default()
        })
        .collect()
}

/// Gives each territory its shape and first owner off the campaign screen's map: a shape
/// named `F` and its number, with its rim `E` and the number in it, painted in its owner's
/// colour.
fn shape_territories(scene: &xbf::Model, territories: &mut [Territory]) {
    let Some(map) = scene.objects.iter().find(|object| object.name == MAP) else {
        return;
    };
    let world = map.transform;
    for shape in &map.children {
        let Some(at) = numbered(&shape.name, SHAPE).filter(|&at| at < territories.len()) else {
            continue;
        };
        let world = xbf::times(&shape.transform, &world);
        let territory = &mut territories[at];
        territory.fill = flat_faces(shape, &world);
        let rim = shape
            .children
            .iter()
            .find(|part| numbered(&part.name, RIM) == Some(at));
        if let Some(rim) = rim {
            territory.rim = flat_faces(rim, &xbf::times(&rim.transform, &world));
            territory.first_owner = rim
                .faces
                .iter()
                .filter_map(|face| scene.textures.get(face.texture?))
                .find_map(|picture| {
                    RIM_PICTURES
                        .iter()
                        .find(|(name, _)| picture.eq_ignore_ascii_case(name))
                        .map(|&(_, house)| house)
                });
        }
        territory.centre = middle(&territory.fill);
    }
}

/// The place, from 0, of a part named by this letter and a number from 1, such as `F07`.
fn numbered(name: &str, letter: char) -> Option<usize> {
    let digits = name.strip_prefix(letter)?;
    (digits.len() == 2).then_some(())?;
    digits.parse::<usize>().ok()?.checked_sub(1)
}

/// A part's faces as the original's camera sees them, flat on its screen.
fn flat_faces(part: &xbf::Object, world: &[[f32; 4]; 4]) -> Vec<[(f32, f32); 3]> {
    let seen = |vertex: usize| {
        let [x, y, z] = xbf::point(world, part.vertices.get(vertex)?.position);
        (z > 0.0).then(|| (x / z * FOCAL, y / z * FOCAL))
    };
    part.faces
        .iter()
        .filter_map(|face| {
            let [a, b, c] = face.corners;
            Some([seen(a)?, seen(b)?, seen(c)?])
        })
        .collect()
}

/// The middle of a shape, each of its triangles counting as much as it is large.
fn middle(triangles: &[[(f32, f32); 3]]) -> Option<(f32, f32)> {
    let (mut area, mut x, mut y) = (0.0, 0.0, 0.0);
    for &[a, b, c] in triangles {
        let size = ((b.0 - a.0) * (c.1 - a.1) - (c.0 - a.0) * (b.1 - a.1)).abs() / 2.0;
        area += size;
        x += size * (a.0 + b.0 + c.0) / 3.0;
        y += size * (a.1 + b.1 + c.1) / 3.0;
    }
    (area > 0.0).then(|| (x / area, y / area))
}

/// The houses' jump points, out of the `#define`s that name them.
fn jump_points(forced: &str) -> Vec<(Faction, usize)> {
    forced
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            (words.next()? == "#define").then_some(())?;
            let name = words.next()?;
            let at = words.next()?.parse().ok()?;
            JUMP_POINTS
                .iter()
                .find(|(named, _)| *named == name)
                .map(|&(_, house)| (house, at))
        })
        .collect()
}

/// A homeworld belongs to the house whose jump point it is reached from.
fn own_homeworlds(territories: &mut [Territory], jump_points: &[(Faction, usize)]) {
    for territory in territories.iter_mut().skip(ARRAKIS) {
        territory.first_owner = territory.neighbours.iter().find_map(|&at| {
            jump_points
                .iter()
                .find(|&&(_, point)| point == at)
                .map(|&(house, _)| house)
        });
    }
}

/// The phases' rules, and what brings each tech level, in order, out of `PhaseRules.txt`.
fn rules(text: &str) -> (Vec<(u32, Phase)>, Vec<Rule>) {
    let (mut phases, mut tech) = (Vec::new(), Vec::new());
    let mut in_tech = false;
    for line in text.lines().map(str::trim) {
        if let Some(head) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let mut words = head.split_whitespace();
            let number = words.next_back().and_then(|n| n.parse().ok());
            in_tech = head.starts_with("Tech Level");
            if !in_tech && let Some(number) = number {
                phases.push((number, Phase::default()));
            }
            continue;
        }
        let mut words = line.split_whitespace();
        let (Some(key), Some(value)) = (words.next(), words.next().and_then(|v| v.parse().ok()))
        else {
            continue;
        };
        if in_tech {
            match key {
                "Phase" => tech.push(Rule::Phase(value)),
                "Captured" => tech.push(Rule::Captured(value)),
                _ => {}
            }
            continue;
        }
        let Some((_, phase)) = phases.last_mut() else {
            continue;
        };
        match key {
            "Battles" => phase.battles = value,
            "Captured" => phase.captured = value,
            "MaxBattles" => phase.most_battles = Some(value),
            "JumpPoint" => phase.jump_point = value > 0,
            "JumpToChoice1" => phase.jump_to = Some(value),
            "Warning" => phase.warning = Some(value),
            "Lose" => phase.lose = Some(value),
            _ => {}
        }
    }
    (phases, tech)
}

/// The new units of each tech level out of `NEWUNITS.INI`: a line with a level's number, then
/// a line for each unit, its model's file and the id the game's text knows it by. `text`
/// gives what the text calls a unit of an id and what it says of it.
fn new_units(
    listed: &str,
    text: impl Fn(&str) -> (Option<String>, Option<String>),
) -> Vec<NewUnit> {
    const HOUSES: [(&str, Option<Faction>); 4] = [
        ("AT_", Some(Faction::Atreides)),
        ("HK_", Some(Faction::Harkonnen)),
        ("OR_", Some(Faction::Ordos)),
        ("G_", None),
    ];
    let mut level = 0;
    let mut units = Vec::new();
    for line in listed.lines() {
        let line = line.split("//").next().unwrap_or("").trim();
        let mut words = line.split_whitespace();
        let (Some(first), second) = (words.next(), words.next()) else {
            continue;
        };
        let Some(id) = second else {
            if let Ok(number) = first.parse() {
                level = number;
            }
            continue;
        };
        let Some(&(_, house)) = HOUSES
            .iter()
            .find(|(prefix, _)| first.to_uppercase().starts_with(prefix))
        else {
            continue;
        };
        let (name, about) = text(id);
        units.push(NewUnit {
            level,
            house,
            name: name.unwrap_or_else(|| id.to_string()),
            about: about.unwrap_or_default(),
        });
    }
    units
}

/// The side the Harkonnen take in their civil war.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Copec,
    Gunseng,
}

/// A battle of the campaign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mission {
    /// The house's first battle, before the map.
    Start,
    /// On a territory, by its place.
    Attack(usize),
    Defend(usize, Faction),
    Heighliner,
    HomeDefence,
    CivilWar(Side),
    /// On this house's homeworld.
    Homeworld(Faction),
    Final,
}

/// What the campaign waits for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// This battle to be fought.
    Fight(Mission),
    /// The player to pick a territory to attack, of `Campaign::targets`.
    Pick,
    /// The player to say whether to defend this territory against this house.
    Defend(usize, Faction),
    /// The Harkonnen player to take a side in their civil war.
    Side,
    Won,
    Lost,
}

/// A campaign under way.
#[derive(Clone, Debug, PartialEq)]
pub struct Campaign {
    pub house: Faction,
    /// Who holds each territory now.
    pub owners: Vec<Faction>,
    pub phase: u32,
    pub tech: u32,
    /// The phases gone through.
    reached: Vec<u32>,
    /// In this phase: the battles fought, the territories taken, and the battles in a row
    /// that gained nothing.
    battles: u32,
    captured: u32,
    barren: u32,
    /// The phase to go back to after a mission jumped to.
    back_to: u32,
    /// The attacks on the player's territories still to be answered.
    attacks: Vec<(usize, Faction)>,
    /// Whether the player has just been warned to gain ground or lose the campaign.
    pub warned: bool,
    over: Option<bool>,
    random: u32,
}

impl Campaign {
    /// A house's campaign as it starts, before its first battle.
    pub fn new(house: Faction, seed: u32) -> Self {
        let owners = data().map_or_else(Vec::new, |data| {
            data.territories
                .iter()
                .map(|territory| territory.first_owner.unwrap_or(house))
                .collect()
        });
        Campaign {
            house,
            owners,
            phase: START_PHASE,
            tech: 0,
            reached: vec![START_PHASE],
            battles: 0,
            captured: 0,
            barren: 0,
            back_to: FIRST_PHASE,
            attacks: Vec::new(),
            warned: false,
            over: None,
            random: seed | 1,
        }
    }

    /// A house's campaign as it is when this battle of this phase comes: the phases before it
    /// gone through, and a territory to defend the player's own and under attack.
    pub fn at(house: Faction, phase: u32, mission: Mission, seed: u32) -> Self {
        let mut campaign = Campaign::new(house, seed);
        for phase in FIRST_PHASE..=phase {
            campaign.enter(phase);
        }
        if let Mission::Defend(at, by) = mission
            && let Some(owner) = campaign.owners.get_mut(at)
        {
            *owner = house;
            campaign.attacks.push((at, by));
        }
        campaign.raise_tech();
        campaign
    }

    /// What the campaign waits for now.
    pub fn next(&self) -> Next {
        if let Some(won) = self.over {
            return if won { Next::Won } else { Next::Lost };
        }
        if let Some(&(at, by)) = self.attacks.first() {
            return Next::Defend(at, by);
        }
        match self.phase {
            START_PHASE => Next::Fight(Mission::Start),
            HEIGHLINER_PHASE => Next::Fight(Mission::Heighliner),
            HOME_DEFENCE_PHASE if self.house == Faction::Harkonnen => Next::Side,
            HOME_DEFENCE_PHASE => Next::Fight(Mission::HomeDefence),
            COPEC_PHASE => Next::Fight(Mission::CivilWar(Side::Copec)),
            GUNSENG_PHASE => Next::Fight(Mission::CivilWar(Side::Gunseng)),
            FINAL_PHASE => Next::Fight(Mission::Final),
            _ => Next::Pick,
        }
    }

    fn on_arrakis(&self) -> bool {
        (FIRST_PHASE..=LAST_ARRAKIS_PHASE).contains(&self.phase)
    }

    fn rule(&self) -> Phase {
        data()
            .and_then(|data| data.phases.iter().find(|(n, _)| *n == self.phase))
            .map_or_else(Phase::default, |&(_, phase)| phase)
    }

    fn is_jump_point(&self, at: usize) -> bool {
        data().is_some_and(|data| data.jump_points.iter().any(|&(_, point)| point == at))
    }

    fn neighbours(&self, at: usize) -> &'static [usize] {
        data()
            .and_then(|data| data.territories.get(at))
            .map_or(&[], |territory| &territory.neighbours)
    }

    /// Whether the player holds the jump point of another house.
    fn holds_enemy_jump_point(&self) -> bool {
        data().is_some_and(|data| {
            data.jump_points
                .iter()
                .any(|&(house, at)| house != self.house && self.owners.get(at) == Some(&self.house))
        })
    }

    /// The territories the player can attack now: on Arrakis those of others next to their
    /// own, a jump point only once a phase asks for one; when the homeworlds are attacked,
    /// those reached from the jump points the player holds.
    pub fn targets(&self) -> Vec<usize> {
        if self.next() != Next::Pick {
            return Vec::new();
        }
        let mine = |at: usize| self.owners.get(at) == Some(&self.house);
        let homeworlds = self.phase == HOMEWORLD_PHASE;
        let jump_points = self.rule().jump_point;
        (0..self.owners.len())
            .filter(|&at| !mine(at) && (at >= ARRAKIS) == homeworlds)
            .filter(|&at| homeworlds || jump_points || !self.is_jump_point(at))
            .filter(|&at| self.neighbours(at).iter().any(|&n| mine(n)))
            .collect()
    }

    /// The battle for attacking this territory, if it can be attacked now.
    pub fn attack(&self, at: usize) -> Option<Mission> {
        self.targets().contains(&at).then(|| {
            if at >= ARRAKIS {
                Mission::Homeworld(self.owners[at])
            } else {
                Mission::Attack(at)
            }
        })
    }

    /// Whom the player fights in a battle.
    pub fn enemy(&self, mission: Mission) -> Faction {
        match mission {
            Mission::Attack(at) => self.owners.get(at).copied().unwrap_or(self.house.next()),
            Mission::Defend(_, by) | Mission::Homeworld(by) => by,
            Mission::Start => self.house.next(),
            _ => self.strongest_rival(),
        }
    }

    /// The other house that holds the most territories, the first after the player's if as
    /// many.
    fn strongest_rival(&self) -> Faction {
        let rivals = [self.house.next(), self.house.next().next()];
        let held = |house: Faction| self.owners.iter().filter(|&&o| o == house).count();
        if held(rivals[1]) > held(rivals[0]) {
            rivals[1]
        } else {
            rivals[0]
        }
    }

    /// The original's map for a battle: a territory's own, or that of the mission.
    pub fn map_id(&self, mission: Mission) -> String {
        let house = |atreides: &str, harkonnen: &str, ordos: &str| match self.house {
            Faction::Atreides => atreides.to_string(),
            Faction::Harkonnen => harkonnen.to_string(),
            Faction::Ordos => ordos.to_string(),
        };
        match mission {
            Mission::Start => house("#u1", "#u3", "#u2"),
            Mission::Attack(at) | Mission::Defend(at, _) => format!("#t{}", at + 1),
            Mission::Heighliner => house("#h1", "#h3", "#h2"),
            Mission::HomeDefence => house("#d1", "#v1", "#d2"),
            Mission::CivilWar(Side::Copec) => "#v1".to_string(),
            Mission::CivilWar(Side::Gunseng) => "#c1".to_string(),
            Mission::Homeworld(Faction::Atreides) => "#a1".to_string(),
            Mission::Homeworld(Faction::Ordos) => "#a2".to_string(),
            Mission::Homeworld(Faction::Harkonnen) => "#a3".to_string(),
            Mission::Final => "#e1".to_string(),
        }
    }

    /// The Harkonnen player takes a side in their civil war, and fights its mission next.
    pub fn take_side(&mut self, side: Side) {
        if self.next() == Next::Side {
            self.enter(match side {
                Side::Copec => COPEC_PHASE,
                Side::Gunseng => GUNSENG_PHASE,
            });
        }
    }

    /// The player lets the territory under attack go without a fight.
    pub fn give_up(&mut self) {
        if let Next::Defend(at, by) = self.next() {
            self.attacks.remove(0);
            self.owners[at] = by;
        }
    }

    /// A battle has been fought, and won or lost: what follows from it.
    pub fn finish(&mut self, mission: Mission, won: bool) {
        self.warned = false;
        match mission {
            Mission::Start => {
                if won {
                    self.enter(FIRST_PHASE);
                }
            }
            Mission::Attack(at) => {
                self.battles += 1;
                if won {
                    self.owners[at] = self.house;
                    self.captured += 1;
                    self.barren = 0;
                } else {
                    self.barren += 1;
                }
                self.move_on();
                if self.on_arrakis() && self.over.is_none() {
                    self.others_attack();
                }
            }
            Mission::Defend(at, by) => {
                if self.attacks.first() == Some(&(at, by)) {
                    self.attacks.remove(0);
                }
                self.battles += 1;
                if !won {
                    self.owners[at] = by;
                }
            }
            _ => {
                if won {
                    self.battles += 1;
                    self.move_on();
                }
            }
        }
        self.raise_tech();
    }

    /// Moves to the next phase once this one's rules are met, or ends the campaign.
    fn move_on(&mut self) {
        let rule = self.rule();
        if let Some(lose) = rule.lose
            && self.barren >= lose
        {
            self.over = Some(false);
            return;
        }
        self.warned = rule.warning == Some(self.barren) && self.barren > 0;
        let done = if rule.jump_point {
            self.holds_enemy_jump_point()
        } else {
            self.battles >= rule.battles && self.captured >= rule.captured
                || rule.most_battles.is_some_and(|most| self.battles >= most)
        };
        if !done {
            return;
        }
        if self.phase == FINAL_PHASE {
            self.over = Some(true);
        } else if let Some(to) = rule.jump_to {
            if self.on_arrakis() {
                self.back_to = self.phase + 1;
            }
            self.enter(to);
        } else if self.on_arrakis() {
            self.enter(self.phase + 1);
        } else {
            self.enter(self.back_to);
        }
    }

    fn enter(&mut self, phase: u32) {
        self.phase = phase;
        self.reached.push(phase);
        self.battles = 0;
        self.captured = 0;
        self.barren = 0;
    }

    /// Raises the tech level for as long as what brings the next is met. One that comes with
    /// territories taken in a phase counts as met once a later phase it waits for is reached.
    fn raise_tech(&mut self) {
        let Some(rules) = data().map(|data| &data.tech) else {
            return;
        };
        while let Some(rule) = rules.get(self.tech as usize) {
            let reached = |rule: &Rule| matches!(rule, Rule::Phase(p) if self.reached.contains(p));
            let met = match rule {
                Rule::Phase(_) => reached(rule),
                Rule::Captured(n) => {
                    self.captured >= *n || rules[self.tech as usize + 1..].iter().any(reached)
                }
            };
            if !met {
                break;
            }
            self.tech += 1;
        }
    }

    /// The other houses' turn: each may attack a territory next to its own, not a jump point.
    /// One of the player's is theirs to defend; one of a third house's is won or lost by
    /// chance.
    fn others_attack(&mut self) {
        for rival in [self.house.next(), self.house.next().next()] {
            if next_random(&mut self.random) >= ATTACK_CHANCE {
                continue;
            }
            let targets: Vec<usize> = (0..ARRAKIS.min(self.owners.len()))
                .filter(|&at| self.owners[at] != rival && !self.is_jump_point(at))
                .filter(|&at| self.neighbours(at).iter().any(|&n| self.owners[n] == rival))
                .collect();
            if targets.is_empty() {
                continue;
            }
            let pick = next_random(&mut self.random) * targets.len() as f32;
            let at = targets[(pick as usize).min(targets.len() - 1)];
            if self.owners[at] == self.house {
                if self.attacks.iter().all(|&(attacked, _)| attacked != at) {
                    self.attacks.push((at, rival));
                }
            } else if next_random(&mut self.random) < AI_WIN_CHANCE {
                self.owners[at] = rival;
            }
        }
    }

    /// What a battle is called on its briefing.
    pub fn title(&self, mission: Mission) -> String {
        let name = |at: usize| {
            data()
                .and_then(|data| data.territories.get(at))
                .map_or_else(String::new, |territory| territory.name.clone())
        };
        match mission {
            Mission::Start => "The First Battle".to_string(),
            Mission::Attack(at) => format!("Attack {}", name(at)),
            Mission::Defend(at, _) => format!("Defend {}", name(at)),
            Mission::Heighliner => "The Heighliner".to_string(),
            Mission::HomeDefence => "Defend the Homeworld".to_string(),
            Mission::CivilWar(Side::Copec) => "Civil War: for Copec".to_string(),
            Mission::CivilWar(Side::Gunseng) => "Civil War: for Gunseng".to_string(),
            Mission::Homeworld(house) => {
                let at = self
                    .owners
                    .iter()
                    .enumerate()
                    .skip(ARRAKIS)
                    .find(|&(at, &owner)| {
                        owner == house
                            && data().is_some_and(|d| d.territories[at].first_owner == Some(house))
                    })
                    .map_or(0, |(at, _)| at);
                format!("Attack {}", name(at))
            }
            Mission::Final => "The Last Battle".to_string(),
        }
    }

    /// The units the new units screen shows for this tech level: the player's house's, and
    /// everybody's.
    pub fn new_units(&self, level: u32) -> Vec<&'static NewUnit> {
        data().map_or_else(Vec::new, |data| {
            data.new_units
                .iter()
                .filter(|unit| unit.level == level && unit.house.is_none_or(|h| h == self.house))
                .collect()
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The disc's connections between the territories, its owners at the start, and its
    /// rules, for the campaign tests play.
    const CONNECTIONS: &str = "2,3\n1,3,4,5\n1,2,5,6\n2,5,7,8\n2,3,4,6,8,9\n3,5,9,10,11\n4,8,12\n\
        4,5,7,9,12,13,14\n5,6,8,10,14,15,16\n6,9,11,16\n6,10,16,17\n7,8,13,18\n8,12,14,18\n\
        8,9,13,15,18,19,20\n9,14,20,21,28,22,16\n9,10,11,15,17,22,23\n11,16,23\n12,13,14,19,24\n\
        14,18,20,24,25,32\n14,15,19,21,25,26,27\n15,20,27,28\n15,16,23,28,29,30\n16,17,22,30\n\
        18,19,32,33\n19,20,26,32\n20,25,27\n20,21,26,28\n15,21,22,27,29\n22,28,30,31\n22,23,29,31\n\
        29,30\n19,24,25,33\n24,32\n1\n31\n33\n";
    const HARKONNEN: [usize; 11] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    const ATREIDES: [usize; 11] = [12, 13, 14, 18, 19, 20, 24, 25, 26, 32, 33];
    const PHASE_RULES: &str = "[Phase 0]\nJumpToChoice1 0\n[Phase 1]\nBattles 2\nCaptured 1\n\
        MaxBattles 2\nJumpToChoice1 10\n[Phase 10]\nBattles 1\n[Phase 2]\nBattles 2\nCaptured 1\n\
        MaxBattles 2\nJumpToChoice1 11\n[Phase 11]\nBattles 1\n[Phase 3]\nJumpPoint 1\n\
        JumpToChoice1 12\nWarning 3\nLose 5\n[Phase 12]\nBattles 1\nJumpToChoice1 13\n\
        [Phase 13]\nBattles 1\n[Phase 14]\nBattles 1\n[Phase 15]\nBattles 1\n\
        [Tech Level 1]\nPhase 1\n[Tech Level 2]\nCaptured 1\n[Tech Level 3]\nPhase 2\n\
        [Tech Level 4]\nCaptured 1\n[Tech Level 5]\nPhase 3\n[Tech Level 6]\nCaptured 1\n\
        [Tech Level 7]\nCaptured 2\n[Tech Level 8]\nPhase 12\n";
    const FORCED: &str =
        "#define\tATJumpPoint\t32\n#define\tHKJumpPoint 0\n#define\tORJumpPoint 30\n";

    pub(crate) fn data() -> Data {
        let mut territories = territories(CONNECTIONS, |n| Some(format!("Test Territory {n}")));
        for (at, territory) in territories.iter_mut().enumerate().take(ARRAKIS) {
            let number = at + 1;
            territory.first_owner = Some(if HARKONNEN.contains(&number) {
                Faction::Harkonnen
            } else if ATREIDES.contains(&number) {
                Faction::Atreides
            } else {
                Faction::Ordos
            });
            territory.centre = Some(((at % 6) as f32 * 60.0, (at / 6) as f32 * 50.0));
        }
        let jump_points = jump_points(FORCED);
        own_homeworlds(&mut territories, &jump_points);
        let (phases, tech) = rules(PHASE_RULES);
        let new_units = new_units(
            "1\nAT_scout.XBF\tATScout\nHK_scout.XBF\tHKScout\n\n2\nG_mcv.XBF\tMCV\n",
            |id| (Some(format!("{id} name")), Some(format!("What {id} is"))),
        );
        // A first battle for the Atreides: their yard at their base, the enemy's palace at
        // theirs, the enemy's side made theirs, an MCV at the first point at update 3, money and a line of speech.
        let start = {
            use Token::{Function as F, Keyword as K, Kind, Speech, Variable as V};
            let text = |t: &str| Token::Text(t.to_string());
            vec![
                vec![K(165), text("("), F(0), text("()"), K(168), text("0)")],
                vec![V(1), K(180), F(18), text("()")],
                vec![
                    K(139),
                    text("("),
                    F(14),
                    text("(),"),
                    Kind(0),
                    text(","),
                    F(4),
                    text("("),
                    F(14),
                    text("()),0,0,1)"),
                ],
                vec![
                    V(2),
                    K(180),
                    K(139),
                    text("("),
                    V(1),
                    text(","),
                    Kind(1),
                    text(","),
                    F(3),
                    text("(),0,0,1)"),
                ],
                // Their side the player's enemy, as the original's scripts make theirs.
                vec![F(60), text("("), V(1), text(","), F(14), text("())")],
                vec![F(62), text("("), F(14), text("(),10000)")],
                vec![F(45), text("("), Speech(642), text(")")],
                vec![K(167)],
                vec![K(165), text("("), F(0), text("()"), K(168), text("3)")],
                vec![
                    F(21),
                    text("("),
                    F(14),
                    text("(),"),
                    Kind(2),
                    text(","),
                    F(5),
                    text("(2))"),
                ],
                vec![K(167)],
                vec![K(165), text("("), F(23), text("("), V(2), text("))")],
                vec![F(83), text("("), K(177), text(")")],
                vec![K(167)],
            ]
        };
        Data {
            territories,
            jump_points,
            phases,
            tech,
            new_units,
            start_scripts: vec![(Faction::Atreides, start)],
            kinds: ["ATConYard", "HKPalace", "MCV", "ATRefinery", "HKConYard"]
                .map(String::from)
                .to_vec(),
            // The one that fails listed first, as the disc may.
            mission_scripts: vec![
                (
                    "ORP3D30TLFail".to_string(),
                    vec![vec![Token::Text("fail".to_string())]],
                ),
                (
                    "ORP3D30TL".to_string(),
                    vec![vec![Token::Text("defend".to_string())]],
                ),
            ],
        }
    }

    /// A campaign past its first battle, on the map of its first phase.
    pub(crate) fn on_the_map(house: Faction) -> Campaign {
        let mut campaign = Campaign::new(house, 7);
        campaign.finish(Mission::Start, true);
        campaign
    }

    #[test]
    fn a_territorys_script_is_named_for_the_house_the_phase_the_battle_and_whom_it_is_with() {
        assert_eq!(
            named_mission("ATP1M7FR"),
            Some((Faction::Atreides, 1, Mission::Attack(6)))
        );
        assert_eq!(
            named_mission("ATP1D7FR"),
            Some((Faction::Atreides, 1, Mission::Defend(6, Faction::Harkonnen))),
            "defended against who held it at first"
        );
        assert_eq!(
            named_mission("HKP2D7SA"),
            Some((Faction::Harkonnen, 2, Mission::Defend(6, Faction::Ordos)))
        );
        assert_eq!(
            named_mission("ORP3M11GN"),
            Some((Faction::Ordos, 3, Mission::Attack(10)))
        );
        for name in ["Atreides Heighliner Mission", "AtreidesHeighlinerMission"] {
            assert_eq!(
                named_mission(name),
                Some((Faction::Atreides, HEIGHLINER_PHASE, Mission::Heighliner))
            );
        }
        assert_eq!(
            named_mission("HKStart"),
            Some((Faction::Harkonnen, START_PHASE, Mission::Start))
        );
        assert_eq!(
            named_mission("hhkheighlinermission"),
            Some((Faction::Harkonnen, HEIGHLINER_PHASE, Mission::Heighliner))
        );
        for other in [
            "ATP1D7FRFail",
            "ATP1D19GNWin",
            "ATP1M7",
            "ATHeighliner",
            "ORJump_reb",
        ] {
            assert_eq!(named_mission(other), None, "{other}");
        }
        assert_eq!(mission_names(), vec!["ORP3D30TL"]);
        let script = mission_script(Faction::Ordos, 3, true, 29).unwrap();
        assert_eq!(script[0], vec![Token::Text("defend".to_string())]);
        assert_eq!(
            mission_script(Faction::Ordos, 3, true, 2),
            None,
            "30 is not 3"
        );
    }

    #[test]
    fn a_campaign_can_be_set_up_at_a_territorys_battle() {
        let campaign = Campaign::at(Faction::Atreides, 1, Mission::Attack(6), 7);
        assert_eq!((campaign.phase, campaign.next()), (1, Next::Pick));
        assert_eq!(campaign.attack(6), Some(Mission::Attack(6)));
        assert_eq!(campaign.tech, on_the_map(Faction::Atreides).tech);

        let defence = Mission::Defend(6, Faction::Harkonnen);
        let mut campaign = Campaign::at(Faction::Atreides, 2, defence, 7);
        assert_eq!(campaign.phase, 2);
        assert_eq!(campaign.owners[6], Faction::Atreides);
        assert_eq!(campaign.next(), Next::Defend(6, Faction::Harkonnen));
        assert_eq!(campaign.enemy(defence), Faction::Harkonnen);
        campaign.finish(defence, true);
        assert_eq!(campaign.next(), Next::Pick);
    }

    #[test]
    fn the_rules_read_as_the_disc_writes_them() {
        let (phases, tech) = rules(PHASE_RULES);
        let phase = |n: u32| phases.iter().find(|(p, _)| *p == n).unwrap().1;
        assert_eq!(
            phase(1),
            Phase {
                battles: 2,
                captured: 1,
                most_battles: Some(2),
                jump_to: Some(10),
                ..Phase::default()
            }
        );
        assert!(phase(3).jump_point);
        assert_eq!((phase(3).warning, phase(3).lose), (Some(3), Some(5)));
        assert_eq!(tech.len(), 8);
        assert_eq!(tech[6], Rule::Captured(2));
        assert_eq!(tech[7], Rule::Phase(12));
    }

    #[test]
    fn territories_touch_as_listed_and_homeworlds_belong_to_their_jump_points_house() {
        let data = data();
        assert_eq!(data.territories.len(), 36);
        assert_eq!(data.territories[0].neighbours, vec![1, 2]);
        assert_eq!(data.territories[33].first_owner, Some(Faction::Harkonnen));
        assert_eq!(data.territories[34].first_owner, Some(Faction::Ordos));
        assert_eq!(data.territories[35].first_owner, Some(Faction::Atreides));
        assert!(data.jump_points.contains(&(Faction::Atreides, 32)));
    }

    #[test]
    fn new_units_come_by_tech_level_and_house() {
        let campaign = Campaign::new(Faction::Atreides, 1);
        let names: Vec<&str> = campaign
            .new_units(1)
            .iter()
            .map(|u| u.name.as_str())
            .collect();
        assert_eq!(names, ["ATScout name"]);
        assert_eq!(campaign.new_units(2)[0].about, "What MCV is");
    }

    #[test]
    fn the_first_battle_won_opens_the_map_at_the_first_tech_level() {
        let mut campaign = Campaign::new(Faction::Atreides, 1);
        assert_eq!(campaign.next(), Next::Fight(Mission::Start));
        campaign.finish(Mission::Start, false);
        assert_eq!(campaign.next(), Next::Fight(Mission::Start), "again");
        campaign.finish(Mission::Start, true);
        assert_eq!(campaign.next(), Next::Pick);
        assert_eq!(campaign.tech, 1);
    }

    #[test]
    fn only_enemy_territories_next_to_the_players_can_be_attacked_and_not_jump_points_at_first() {
        let campaign = on_the_map(Faction::Atreides);
        let targets = campaign.targets();
        assert!(!targets.is_empty());
        for &at in &targets {
            assert_ne!(campaign.owners[at], Faction::Atreides);
            assert!(at < ARRAKIS);
            assert!(!campaign.is_jump_point(at));
            assert!(
                campaign
                    .neighbours(at)
                    .iter()
                    .any(|&n| campaign.owners[n] == Faction::Atreides)
            );
        }
        // Territory 9 is Harkonnen and touches the Atreides' 14.
        assert!(targets.contains(&8));
        assert_eq!(campaign.attack(8), Some(Mission::Attack(8)));
        assert_eq!(campaign.enemy(Mission::Attack(8)), Faction::Harkonnen);
        assert_eq!(campaign.map_id(Mission::Attack(8)), "#t9");
        assert_eq!(
            campaign.attack(0),
            None,
            "the Harkonnen jump point, far off"
        );
    }

    #[test]
    fn a_won_attack_takes_the_territory_and_two_battles_move_on_to_the_heighliner() {
        let mut campaign = on_the_map(Faction::Atreides);
        campaign.finish(Mission::Attack(8), true);
        assert_eq!(campaign.owners[8], Faction::Atreides);
        assert_eq!(campaign.tech, 2, "a territory taken");
        while let Next::Defend(..) = campaign.next() {
            campaign.give_up();
        }
        let at = campaign.targets()[0];
        campaign.finish(Mission::Attack(at), false);
        assert_eq!(campaign.phase, HEIGHLINER_PHASE);
        assert_eq!(campaign.next(), Next::Fight(Mission::Heighliner));
        assert_eq!(campaign.map_id(Mission::Heighliner), "#h1");
        campaign.finish(Mission::Heighliner, false);
        assert_eq!(
            campaign.next(),
            Next::Fight(Mission::Heighliner),
            "until it is won"
        );
        campaign.finish(Mission::Heighliner, true);
        assert_eq!(campaign.phase, 2);
        assert_eq!(campaign.tech, 3);
    }

    #[test]
    fn the_other_houses_attack_and_the_player_defends_or_gives_up() {
        let mut campaign = on_the_map(Faction::Atreides);
        let mut asked = false;
        for _ in 0..20 {
            campaign.attacks.clear();
            campaign.others_attack();
            if let Next::Defend(at, by) = campaign.next() {
                asked = true;
                assert_eq!(campaign.owners[at], Faction::Atreides);
                assert_ne!(by, Faction::Atreides);
                assert!(!campaign.is_jump_point(at));
                campaign.finish(Mission::Defend(at, by), true);
                assert_eq!(campaign.owners[at], Faction::Atreides, "held");
                if let Next::Defend(at, by) = campaign.next() {
                    campaign.give_up();
                    assert_eq!(campaign.owners[at], by, "given up");
                }
                assert_eq!(campaign.next(), Next::Pick);
            }
        }
        assert!(asked);
    }

    #[test]
    fn the_harkonnen_take_a_side_where_the_others_defend_their_homeworld() {
        let mut campaign = on_the_map(Faction::Harkonnen);
        campaign.enter(HOME_DEFENCE_PHASE);
        assert_eq!(campaign.next(), Next::Side);
        campaign.take_side(Side::Gunseng);
        assert_eq!(
            campaign.next(),
            Next::Fight(Mission::CivilWar(Side::Gunseng))
        );
        assert_eq!(campaign.map_id(Mission::CivilWar(Side::Gunseng)), "#c1");
        let mut atreides = on_the_map(Faction::Atreides);
        atreides.enter(HOME_DEFENCE_PHASE);
        assert_eq!(atreides.next(), Next::Fight(Mission::HomeDefence));
        assert_eq!(atreides.map_id(Mission::HomeDefence), "#d1");
    }

    #[test]
    fn a_jump_point_taken_leads_to_its_homeworld_and_then_the_last_battle() {
        let mut campaign = on_the_map(Faction::Atreides);
        campaign.enter(2);
        campaign.enter(LAST_ARRAKIS_PHASE);
        assert!(
            !campaign.targets().contains(&30),
            "not next to the player yet"
        );
        for at in [21, 27, 28, 29] {
            campaign.owners[at] = Faction::Atreides;
        }
        assert!(campaign.targets().contains(&30), "the Ordos jump point");
        campaign.finish(Mission::Attack(30), true);
        assert_eq!(campaign.phase, HOMEWORLD_PHASE);
        campaign.attacks.clear();
        assert_eq!(campaign.next(), Next::Pick);
        assert_eq!(campaign.targets(), vec![34], "Draconis IV, reached from it");
        let mission = campaign.attack(34).unwrap();
        assert_eq!(mission, Mission::Homeworld(Faction::Ordos));
        assert_eq!(campaign.map_id(mission), "#a2");
        campaign.finish(mission, true);
        assert_eq!(campaign.next(), Next::Fight(Mission::Final));
        assert_eq!(campaign.map_id(Mission::Final), "#e1");
        assert_eq!(campaign.tech, 8);
        campaign.finish(Mission::Final, true);
        assert_eq!(campaign.next(), Next::Won);
    }

    #[test]
    fn too_many_battles_without_ground_gained_warn_and_then_lose_the_campaign() {
        let mut campaign = on_the_map(Faction::Atreides);
        campaign.enter(LAST_ARRAKIS_PHASE);
        for n in 1..=5 {
            campaign.attacks.clear();
            let at = campaign.targets()[0];
            campaign.finish(Mission::Attack(at), false);
            assert_eq!(campaign.warned, n == 3, "after {n}");
        }
        assert_eq!(campaign.next(), Next::Lost);
    }

    #[test]
    fn the_disc_has_arrakis_its_owners_shapes_rules_and_new_units() {
        let Some(data) = read(Path::new("/Volumes/EMPEROR1")) else {
            return;
        };
        assert_eq!(data.territories.len(), 36);
        let arrakis = &data.territories[..ARRAKIS];
        for house in [Faction::Atreides, Faction::Harkonnen, Faction::Ordos] {
            let held = arrakis
                .iter()
                .filter(|t| t.first_owner == Some(house))
                .count();
            assert_eq!(held, 11, "{house:?}");
        }
        assert!(
            arrakis
                .iter()
                .all(|t| !t.fill.is_empty() && t.centre.is_some())
        );
        assert_eq!(data.territories[0].name, "Harkonnen Stronghold");
        assert_eq!(data.territories[35].name, "Caladan");
        assert_eq!(data.territories[35].first_owner, Some(Faction::Atreides));
        assert_eq!(data.phases, rules(PHASE_RULES).0);
        let scout = data.new_units.iter().find(|u| u.level == 1).unwrap();
        assert_eq!(scout.house, Some(Faction::Atreides));
        assert!(!scout.about.is_empty(), "{scout:?}");
        let script = &data
            .start_scripts
            .iter()
            .find(|(h, _)| *h == Faction::Atreides)
            .unwrap()
            .1;
        assert_eq!(script.len(), 754);
        assert_eq!(data.kinds[27], "ATConYard");
        assert_eq!(
            kinds("[UnitTypes]\nA\n[Other]\nB\n[UnitTypes]\nC // c\n"),
            ["A", "C"]
        );
        // Run on its own, the script puts down both bases at once and the Mentat speaks first;
        // the units it brings at its 49th update are a gift to a co-op game's second player,
        // and this is no co-op game.
        struct Written(i32, Vec<String>, Vec<String>, Vec<u16>, i32, Vec<String>);
        impl mission::Battle for Written {
            fn now(&self) -> i32 {
                self.0
            }
            fn new_side(&mut self) -> i32 {
                self.4 += 1;
                self.4
            }
            fn base(&self, _: i32) -> (f32, f32) {
                (0.0, 0.0)
            }
            fn enemy_base(&self) -> (f32, f32) {
                (0.0, 0.0)
            }
            fn script_point(&self, _: i32) -> (f32, f32) {
                (0.0, 0.0)
            }
            fn place(&mut self, _: i32, kind: u16, _: (f32, f32), _: i32) -> Option<u32> {
                self.1.push(self.5[usize::from(kind)].clone());
                Some(self.1.len() as u32)
            }
            fn bring(&mut self, _: i32, kind: u16, _: (f32, f32)) -> Option<u32> {
                self.2.push(self.5[usize::from(kind)].clone());
                None
            }
            fn left(&self, _: i32) -> i32 {
                10
            }
            fn destroyed(&self, _: Option<u32>) -> bool {
                false
            }
            fn say(&mut self, line: u16) {
                self.3.push(line);
            }
            fn pay(&mut self, _: i32, _: i32) {}
            fn win(&mut self) {}
        }
        let mut script = mission::Script::new(script);
        let mut written = Written(0, Vec::new(), Vec::new(), Vec::new(), 0, data.kinds.clone());
        script.run(&mut written);
        assert!(written.1.iter().any(|k| k == "ATConYard"));
        assert!(written.1.iter().any(|k| k == "HKPalace"));
        assert!(written.1.len() > 60, "{}", written.1.len());
        assert_eq!(written.3, [642]);
        let brought = written.2.len();
        written.0 = 49;
        script.run(&mut written);
        assert_eq!(&written.2[brought..], [] as [&str; 0]);
        // The Atreides start next to the Harkonnen, in the south-west of the map.
        let ((left, bottom), (right, top)) = data.bounds().unwrap();
        assert!(left < right && bottom < top);
    }
}
