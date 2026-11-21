//! The original's sounds on its disc: kept in `AUDIO.BAG`, and named by its sound tables, the
//! `Sfx` files in the cabinet. A table's section lists the sounds one of which plays for
//! something: `[ATKindjalSelection]` lists what a Kindjal says when it is selected, and
//! `[ATGroupSelection]` what a group of Atreides says.

use crate::archive;
use crate::game::{Faction, Said, Speech};
use crate::music::{self, Track};
use crate::units::{self, Role};
use crate::video;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

const CABINET: &str = "INSTALL/Game1.cab";
const BAG: &str = "INSTALL/DATA/SFX/AUDIO.BAG";
/// The bag of what the battlefield says to the player, on the disc that installs the game.
const DIALOG_BAG: &str = "INSTALL/DATA/DIALOG/DIALOG.BAG";
/// The table of what is spoken, in the game's text: a mission's script names a line by its
/// place among the entries of the table's `MISSION_MESSAGES` sections, the first being
/// `FIRST_SPOKEN`; blank lines, comments, briefings and debriefings are not counted. Its names
/// are those of the recordings of `sounds.txt`, whose names in the bags leave off the
/// language's letter.
const TEXT_INDEX: &str = "\\data\\STRINGS0001.RFH";
const TEXT_DATA: &str = "\\data\\STRINGS0001.RFD";
const SPOKEN: &str = "E_Output_Pickup.txt";
const FIRST_SPOKEN: u16 = 502;
/// What the table's sections of what missions say start with, and what every section ends with.
const MISSION_MESSAGES: &str = "MissionMessages";
const SECTION_END: &str = "[END]";
const RECORDINGS: &str = "\\data\\Sounds\\sounds.txt";
const LANGUAGE: char = 'E';
/// A house disc's bag of what is said in its missions.
const HOUSE_DIALOG_BAG: &str = "DIALOG.BAG";
/// The sound tables in the cabinet.
const TABLES: [&str; 4] = [
    "\\data\\Sfx\\AtreidesSFX.txt",
    "\\data\\Sfx\\HarkonnenSFX.txt",
    "\\data\\Sfx\\ORDOSSFX.TXT",
    "\\data\\Sfx\\GeneralSFX.txt",
];
/// What a section's lines naming its sounds, how loud they play and how many may play at once
/// start with.
const SOUNDS: &str = "sounds";
const VOLUME: &str = "volume";
const LIMIT: &str = "limit";
/// How loud a section plays, out of a hundred, and how many of its sounds may play at once,
/// where it does not say: the original's `[GlobalDefaults]` in `Sfx.txt`.
const DEFAULT_VOLUME: f32 = 80.0;
const DEFAULT_LIMIT: usize = 5;

/// What the name of a section of what is said ends with.
fn ending(said: Said) -> &'static str {
    match said {
        Said::Selected => "Selection",
        Said::Moving => "Move",
        Said::Attacking => "Attack",
        // As an MCV, a mortar or a Kindjal says it deploys, and a mortar or a Kindjal that it
        // packs up.
        Said::Deploying => "Special0",
        Said::PackingUp => "Special1",
    }
}

/// A sound to play once: mono or stereo samples, interleaved.
pub struct Line {
    pub samples: Vec<f32>,
    pub channels: u32,
    pub rate: u32,
}

impl Line {
    /// How long it plays.
    pub fn seconds(&self) -> f32 {
        self.samples.len() as f32 / (self.channels * self.rate).max(1) as f32
    }
}

/// One of the sounds a section lists.
#[derive(Debug, PartialEq)]
struct Sound {
    /// Its name in the bag, in lower case.
    name: String,
    /// Whether it is speech, which a `$` marks.
    speech: bool,
}

/// One of the tables' sections: the sounds one of which plays for it, and how loud and how
/// many at once, if it says.
#[derive(Debug, Default, PartialEq)]
struct Section {
    sounds: Vec<Sound>,
    volume: Option<f32>,
    limit: Option<usize>,
}

struct Sounds {
    bag: PathBuf,
    /// The bag's sounds, by their names in lower case.
    tracks: HashMap<String, Track>,
    /// The tables' sections, by their names in lower case.
    sections: HashMap<String, Section>,
    /// The sounds read out of the bag so far, by their names.
    read: Mutex<HashMap<String, Arc<Line>>>,
}

static SOUNDS_READ: OnceLock<Sounds> = OnceLock::new();
static DIALOG_READ: OnceLock<Sounds> = OnceLock::new();
/// The house discs' dialogue, read once they are found.
static HOUSE_DIALOG: OnceLock<Vec<Sounds>> = OnceLock::new();
/// The recording of each line of what is spoken, by the line's place in its table: none for
/// a line that names none.
static SPOKEN_LINES: OnceLock<Vec<Option<String>>> = OnceLock::new();
/// What each line of what is spoken says, by its place in its table.
static SPOKEN_TEXTS: OnceLock<Vec<Option<String>>> = OnceLock::new();

/// Reads the sounds and their tables from the disc mounted at `disc`. Without it, nothing
/// plays.
pub fn load(disc: &Path) {
    let bag = disc.join(BAG);
    let Some(tracks) = music::list(&bag) else {
        return;
    };
    let cabinet = disc.join(CABINET);
    let mut sections = HashMap::new();
    for table in TABLES {
        if let Some(text) = archive::from_cabinet(&cabinet, table) {
            read_table(&String::from_utf8_lossy(&text), &mut sections);
        }
    }
    let tracks = tracks
        .into_iter()
        .map(|track| (track.name.to_lowercase(), track))
        .collect();
    let _ = SOUNDS_READ.set(Sounds {
        bag,
        tracks,
        sections,
        read: Mutex::default(),
    });
    let (recorded, written) = spoken_lines(&cabinet).unwrap_or_default();
    let _ = SPOKEN_LINES.set(recorded);
    let _ = SPOKEN_TEXTS.set(written);
    let dialog = disc.join(DIALOG_BAG);
    if let Some(tracks) = music::list(&dialog) {
        let _ = DIALOG_READ.set(Sounds {
            bag: dialog,
            tracks: tracks
                .into_iter()
                .map(|track| (track.name.to_lowercase(), track))
                .collect(),
            sections: HashMap::new(),
            read: Mutex::default(),
        });
    }
}

/// Reads the dialogue bags of these house discs.
pub fn load_house_dialog(discs: &[PathBuf]) {
    let bags = discs
        .iter()
        .map(|disc| disc.join(HOUSE_DIALOG_BAG))
        .filter_map(|bag| {
            let tracks = music::list(&bag)?;
            Some(Sounds {
                bag,
                tracks: tracks
                    .into_iter()
                    .map(|track| (track.name.to_lowercase(), track))
                    .collect(),
                sections: HashMap::new(),
                read: Mutex::default(),
            })
        })
        .collect();
    let _ = HOUSE_DIALOG.set(bags);
}

/// The recording of each line of the table of what is spoken, as the cabinet has them, and
/// what each says.
fn spoken_lines(cabinet: &Path) -> Option<(Vec<Option<String>>, Vec<Option<String>>)> {
    let index = archive::from_cabinet(cabinet, TEXT_INDEX)?;
    let data = archive::from_cabinet(cabinet, TEXT_DATA)?;
    let spoken = crate::maps::utf16(&archive::read_in(&index, &data, SPOKEN)?);
    let recordings = archive::from_cabinet(cabinet, RECORDINGS)?;
    let recorded = recorded(&spoken, &String::from_utf8_lossy(&recordings));
    Some((recorded, written(&spoken)))
}

/// The recording of each entry of a table of what is spoken that a mission's script can name,
/// by its name in `sounds.txt`'s list of names and recordings, without the language's letter:
/// the entries of its sections of what missions say, in order.
fn recorded(spoken: &str, recordings: &str) -> Vec<Option<String>> {
    let recording = |(id, _): (&str, &str)| {
        recordings.lines().find_map(|line| {
            let (name, file) = line.split_once('\t')?;
            name.trim().eq_ignore_ascii_case(id).then(|| {
                let file = file.trim();
                file.strip_suffix(LANGUAGE).unwrap_or(file).to_string()
            })
        })
    };
    mission_entries(spoken).map(recording).collect()
}

/// What each entry of a table of what is spoken that a mission's script can name says, as
/// the table writes it after its name, between braces.
fn written(spoken: &str) -> Vec<Option<String>> {
    mission_entries(spoken)
        .map(|(_, text)| {
            let (_, text) = text.split_once('{')?;
            let (text, _) = text.rsplit_once('}')?;
            Some(text.split_whitespace().collect::<Vec<_>>().join(" "))
        })
        .collect()
}

/// The entries of the sections of what missions say of a table of what is spoken, in order:
/// each its name and what follows it.
fn mission_entries(spoken: &str) -> impl Iterator<Item = (&str, &str)> {
    let mut inside = false;
    spoken
        .lines()
        .map(|line| {
            let (id, rest) = line.split_once('\t').unwrap_or((line, ""));
            (id.trim(), rest)
        })
        .filter(|(id, _)| !id.is_empty() && !id.starts_with(';'))
        .filter(move |&(id, _)| {
            if id.eq_ignore_ascii_case(MISSION_MESSAGES) {
                inside = true;
            } else if id.eq_ignore_ascii_case(SECTION_END) {
                inside = false;
            } else {
                return inside;
            }
            false
        })
}

/// What the line of speech a mission's script names by this number says, as the game's
/// text writes it: "Mentat : Welcome Commander, …".
pub fn mission_text(number: u16) -> Option<&'static str> {
    let at = usize::from(number.checked_sub(FIRST_SPOKEN)?);
    SPOKEN_TEXTS.get()?.get(at)?.as_deref()
}

/// The line of speech a mission's script names by this number, from whichever disc has it.
pub fn mission_line(number: u16) -> Option<Arc<Line>> {
    let at = usize::from(number.checked_sub(FIRST_SPOKEN)?);
    let name = SPOKEN_LINES.get()?.get(at)?.as_ref()?.to_lowercase();
    let first = DIALOG_READ.get().into_iter();
    let houses = HOUSE_DIALOG.get().into_iter().flatten();
    first
        .chain(houses)
        .find(|sounds| sounds.tracks.contains_key(&name))
        .and_then(|sounds| cached_line(sounds, &name))
}

/// The line of the dialogue bag said for this to a player of this house: the sound the
/// original's `UI_Comp.txt` gives for it.
fn spoken_name(speech: Speech, house: Faction) -> &'static str {
    use Faction::{Atreides, Harkonnen, Ordos};
    match (speech, house) {
        (Speech::UnitReady, _) => "ui-g024",
        (Speech::Training, _) => "ui-g046",
        (Speech::ConstructionStarted, _) => "ui-g030",
        (Speech::ReadyToPlace, Atreides) => "ui-g020",
        (Speech::ReadyToPlace, _) => "ui-g022",
        (Speech::ConstructionComplete, _) => "ui-g174",
        (Speech::UnitLost, _) => "ui-g008",
        (Speech::BuildingLost, _) => "ui-g010",
        (Speech::BuildingSold, _) => "ui-g018",
        (Speech::BaseUnderAttack, _) => "ui-g006",
        (Speech::HarvesterUnderAttack, _) => "ui-g004",
        (Speech::LowPower, _) => "ui-g316",
        (Speech::InsufficientFunds, _) => "ui-g162",
        (Speech::Upgrading, _) => "ui-g106",
        (Speech::GuardOn, _) => "ui-g110",
        (Speech::GuardOff, _) => "ui-g416",
        (Speech::WaypointPlaced, _) => "ui-g124",
        (Speech::WaypointAdded, _) => "ui-g134",
        (Speech::Retreat, _) => "ui-g146",
        (Speech::SuperweaponReady, Atreides) => "ui-g054",
        (Speech::SuperweaponReady, Harkonnen) => "ui-g056",
        (Speech::SuperweaponReady, Ordos) => "ui-g058",
        (Speech::Victory, _) => "ui-g482",
        (Speech::Defeat, _) => "ui-g484",
    }
}

/// What the battlefield says for this to a player of this house, once the disc's dialogue is
/// read.
pub fn spoken(speech: Speech, house: Faction) -> Option<Arc<Line>> {
    cached_line(DIALOG_READ.get()?, spoken_name(speech, house))
}

/// One of the lines these units say about this, as the disc has them: what the one unit, or
/// a group of one kind, says, or else what a group of its house says, or failing that what
/// the first of them says. `pick` chooses among a section's lines, which are its speech if it
/// has any: an aircraft's section has radio static too.
pub fn reply(said: Said, speakers: &[(Role, Faction)], pick: usize) -> Option<Line> {
    let sounds = SOUNDS_READ.get()?;
    let section = sections_for(said, speakers)
        .into_iter()
        .find_map(|name| sounds.sections.get(&name.to_lowercase()))?;
    let speech: Vec<&Sound> = section.sounds.iter().filter(|sound| sound.speech).collect();
    let lines = if speech.is_empty() {
        section.sounds.iter().collect()
    } else {
        speech
    };
    let sound = lines.get(pick % lines.len().max(1))?;
    read_line(sounds, &sound.name)
}

/// The sound of this name out of the bag.
fn read_line(sounds: &Sounds, name: &str) -> Option<Line> {
    let track = sounds.tracks.get(name)?;
    if track.format & Track::MP3 != 0 {
        let samples = video::decode_sound(&track.location(&sounds.bag));
        return (!samples.is_empty()).then_some(Line {
            samples,
            channels: video::SOUND_CHANNELS,
            rate: video::SOUND_RATE,
        });
    }
    let mut bytes = vec![0; track.size as usize];
    let mut bag = File::open(&sounds.bag).ok()?;
    bag.seek(SeekFrom::Start(u64::from(track.offset))).ok()?;
    bag.read_exact(&mut bytes).ok()?;
    Some(Line {
        samples: decode(&bytes, track.format, track.block as usize),
        channels: if track.format & Track::STEREO != 0 {
            2
        } else {
            1
        },
        rate: track.rate,
    })
}

/// A sound of the battlefield to play once, and how loud, from 0 to 1.
pub struct Effect {
    pub line: Arc<Line>,
    pub volume: f32,
}

/// The battlefield's sounds: those still to come, each by its section and when it plays, and
/// those playing, each by its section and when it ends, in seconds of its own clock.
#[derive(Default)]
pub struct Effects {
    clock: f32,
    waiting: Vec<(f32, String)>,
    playing: Vec<(String, f32)>,
    picks: usize,
}

impl Effects {
    /// Sounds to play, each by its section and how many seconds from now.
    pub fn hear(&mut self, sounds: Vec<(f32, String)>) {
        let clock = self.clock;
        self.waiting.extend(
            sounds
                .into_iter()
                .map(|(after, name)| (clock + after, name.to_lowercase())),
        );
    }

    /// Lets this many seconds go by, and gives what is due to play now: one of each due
    /// section's sounds, as loud as it says, unless as many as it allows are playing already.
    pub fn due(&mut self, seconds: f32) -> Vec<Effect> {
        self.clock += seconds;
        let clock = self.clock;
        self.playing.retain(|&(_, ends)| ends > clock);
        let (due, waiting) = std::mem::take(&mut self.waiting)
            .into_iter()
            .partition(|&(at, _)| at <= clock);
        self.waiting = waiting;
        let Some(sounds) = SOUNDS_READ.get() else {
            return Vec::new();
        };
        let mut effects = Vec::new();
        for (_, name) in due {
            let Some(section) = sounds.sections.get(&name) else {
                continue;
            };
            let playing = self.playing.iter().filter(|(n, _)| *n == name).count();
            if section.sounds.is_empty() || playing >= section.limit.unwrap_or(DEFAULT_LIMIT) {
                continue;
            }
            self.picks = self.picks.wrapping_add(1);
            let sound = &section.sounds[self.picks % section.sounds.len()];
            let Some(line) = cached_line(sounds, &sound.name) else {
                continue;
            };
            let lasts = line.samples.len() as f32 / (line.channels * line.rate).max(1) as f32;
            self.playing.push((name, clock + lasts));
            effects.push(Effect {
                line,
                volume: section.volume.unwrap_or(DEFAULT_VOLUME) / 100.0,
            });
        }
        effects
    }
}

/// The sound of this name, read out of the bag the first time it is wanted.
fn cached_line(sounds: &Sounds, name: &str) -> Option<Arc<Line>> {
    let mut read = sounds.read.lock().ok()?;
    if let Some(line) = read.get(name) {
        return Some(line.clone());
    }
    let line = Arc::new(read_line(sounds, name)?);
    read.insert(name.to_string(), line.clone());
    Some(line)
}

/// The sections that might hold what these units say, the likeliest first.
fn sections_for(said: Said, speakers: &[(Role, Faction)]) -> Vec<String> {
    let ending = ending(said);
    let Some(&(role, house)) = speakers.first() else {
        return Vec::new();
    };
    let own = format!("{}{ending}", role.rules_name(house));
    if speakers.iter().all(|&(other, _)| other == role) {
        return vec![own];
    }
    vec![format!("{}Group{ending}", units::house_letters(house)), own]
}

/// Adds the sections of a sound table to these: for each `[name]`, the sounds its `Sounds =`
/// lines name, past a `$` that marks speech, and how loud and how many at once its `Volume =`
/// and `Limit =` say, each up to a `;` that starts a remark.
fn read_table(text: &str, sections: &mut HashMap<String, Section>) {
    let mut section: Option<String> = None;
    for line in text.split(['\r', '\n']) {
        let line = line.split(';').next().unwrap_or("").trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = Some(name.trim().to_lowercase());
        } else if let (Some(section), Some((key, value))) = (&section, line.split_once('=')) {
            let key = key.trim();
            let entry = sections.entry(section.clone()).or_default();
            if key.eq_ignore_ascii_case(SOUNDS) {
                entry
                    .sounds
                    .extend(value.split_whitespace().map(|name| Sound {
                        name: name.trim_start_matches('$').to_lowercase(),
                        speech: name.starts_with('$'),
                    }));
            } else if key.eq_ignore_ascii_case(VOLUME) {
                entry.volume = value.trim().parse().ok();
            } else if key.eq_ignore_ascii_case(LIMIT) {
                entry.limit = value.trim().parse().ok();
            }
        }
    }
}

/// The samples of a sound kept in a bag in this format, from -1 to 1. A packed one is IMA
/// ADPCM in blocks of `block` bytes, each starting afresh from its first sample and step.
fn decode(bytes: &[u8], format: u32, block: usize) -> Vec<f32> {
    let level = |sample: i16| f32::from(sample) / 32768.0;
    if format & Track::PACKED != 0 {
        bytes
            .chunks(block.max(Adpcm::HEAD + 1))
            .filter(|block| block.len() > Adpcm::HEAD)
            .flat_map(|block| {
                let mut adpcm = Adpcm {
                    sample: i16::from_le_bytes([block[0], block[1]]),
                    step: usize::from(block[2]).min(Adpcm::STEPS.len() - 1),
                };
                let first = adpcm.sample;
                std::iter::once(first).chain(
                    block[Adpcm::HEAD..]
                        .iter()
                        .flat_map(|&byte| [byte & 0x0f, byte >> 4])
                        .map(move |nibble| adpcm.next(nibble)),
                )
            })
            .map(level)
            .collect()
    } else if format & Track::SIXTEEN_BITS != 0 {
        bytes
            .chunks_exact(2)
            .map(|pair| level(i16::from_le_bytes([pair[0], pair[1]])))
            .collect()
    } else {
        bytes
            .iter()
            .map(|&byte| (f32::from(byte) - 128.0) / 128.0)
            .collect()
    }
}

/// Where an IMA ADPCM block has got to: its last sample and which of its steps it is at.
struct Adpcm {
    sample: i16,
    step: usize,
}

impl Adpcm {
    /// How many bytes start a block: its first sample, its first step, and one unused.
    const HEAD: usize = 4;
    const STEP_CHANGES: [isize; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];
    const STEPS: [i32; 89] = [
        7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60,
        66, 73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371,
        408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878,
        2066, 2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845,
        8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086,
        29794, 32767,
    ];

    /// The next sample, from the next four bits: a step of an eighth of the step size, and
    /// a half, a quarter or a whole more for each bit of its size.
    fn next(&mut self, nibble: u8) -> i16 {
        let step = Self::STEPS[self.step];
        let size = nibble & 0b0111;
        let change = [(4, step), (2, step >> 1), (1, step >> 2)]
            .iter()
            .filter(|&&(bit, _)| size & bit != 0)
            .fold(step >> 3, |change, &(_, more)| change + more);
        let change = if nibble & 0b1000 != 0 {
            -change
        } else {
            change
        };
        self.sample =
            (i32::from(self.sample) + change).clamp(i16::MIN.into(), i16::MAX.into()) as i16;
        self.step = self
            .step
            .saturating_add_signed(Self::STEP_CHANGES[usize::from(size)])
            .min(Self::STEPS.len() - 1);
        self.sample
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_lists_each_sections_sounds_past_their_marks_and_remarks() {
        let mut sections = HashMap::new();
        read_table(
            "[LocalDefaults]\r\nType\t=\tDialog\r\n\r\n[ATKindjalSelection]\r\nSounds = $00-US02\t\t ;\"Awaiting orders.\"\r\nSounds = $00-US04\r\n;Sounds = $00-US06\r[hkcrush]\rSounds=crush_guy_1 crush_guy_2\rLimit = 1\rVolume=70 ;loud\r",
            &mut sections,
        );
        let names = |section: &str| -> Vec<(&str, bool)> {
            sections[section]
                .sounds
                .iter()
                .map(|sound| (sound.name.as_str(), sound.speech))
                .collect()
        };
        assert_eq!(
            names("atkindjalselection"),
            [("00-us02", true), ("00-us04", true)]
        );
        assert_eq!(
            names("hkcrush"),
            [("crush_guy_1", false), ("crush_guy_2", false)]
        );
        assert!(sections["localdefaults"].sounds.is_empty());
        assert_eq!(sections["hkcrush"].volume, Some(70.0));
        assert_eq!(sections["hkcrush"].limit, Some(1));
    }

    #[test]
    fn one_unit_or_one_kind_says_its_own_and_a_mixed_group_its_houses() {
        use Faction::{Atreides, Harkonnen};
        assert_eq!(
            sections_for(Said::Selected, &[(Role::AtKindjal, Atreides)]),
            ["ATKindjalSelection"]
        );
        assert_eq!(
            sections_for(Said::Deploying, &[(Role::AtKindjal, Atreides)]),
            ["ATKindjalSpecial0"]
        );
        assert_eq!(
            sections_for(Said::PackingUp, &[(Role::AtKindjal, Atreides)]),
            ["ATKindjalSpecial1"]
        );
        assert_eq!(
            sections_for(
                Said::Moving,
                &[(Role::Harvester, Harkonnen), (Role::Harvester, Harkonnen)]
            ),
            ["HKHarvesterMove"]
        );
        assert_eq!(
            sections_for(
                Said::Attacking,
                &[(Role::HkAssault, Harkonnen), (Role::HkTrooper, Harkonnen)]
            ),
            ["HKGroupAttack", "HKAssaultAttack"]
        );
        assert!(sections_for(Said::Selected, &[]).is_empty());
    }

    #[test]
    fn sounds_decode_from_sixteen_bits_and_from_four_bit_steps() {
        assert_eq!(
            decode(&[0x00, 0x40, 0x00, 0xc0], Track::SIXTEEN_BITS, 0),
            [0.5, -0.5]
        );
        let packed = Track::SIXTEEN_BITS | Track::PACKED;
        // A block's first sample, then two from each byte: up by the step size and more, then
        // a bigger step up, then down.
        let block = [0x00, 0x10, 0, 0, 0x77, 0x0f];
        let samples = decode(&block, packed, 6);
        let levels: Vec<i32> = samples.iter().map(|&s| (s * 32768.0) as i32).collect();
        assert_eq!(&levels[..3], [4096, 4096 + 11, 4096 + 11 + 30]);
        assert!(levels[3] < levels[2]);
        // The next block starts afresh.
        let twice = decode(&[block, block].concat(), packed, 6);
        assert_eq!(twice[5..], samples[..]);
    }

    #[test]
    fn every_spoken_message_is_on_the_disc_when_it_is_mounted() {
        let disc = Path::new("/Volumes/EMPEROR1");
        let Some(tracks) = music::list(&disc.join(DIALOG_BAG)) else {
            return;
        };
        let names: Vec<String> = tracks.iter().map(|t| t.name.to_lowercase()).collect();
        let all = [
            Speech::UnitReady,
            Speech::Training,
            Speech::ConstructionStarted,
            Speech::ReadyToPlace,
            Speech::ConstructionComplete,
            Speech::UnitLost,
            Speech::BuildingLost,
            Speech::BuildingSold,
            Speech::BaseUnderAttack,
            Speech::HarvesterUnderAttack,
            Speech::LowPower,
            Speech::InsufficientFunds,
            Speech::Upgrading,
            Speech::GuardOn,
            Speech::GuardOff,
            Speech::WaypointPlaced,
            Speech::WaypointAdded,
            Speech::Retreat,
            Speech::SuperweaponReady,
            Speech::Victory,
            Speech::Defeat,
        ];
        for speech in all {
            for house in [Faction::Atreides, Faction::Harkonnen, Faction::Ordos] {
                let name = spoken_name(speech, house);
                assert!(
                    names.iter().any(|n| n == name),
                    "{speech:?} {house:?}: {name}"
                );
            }
        }
    }

    #[test]
    fn a_scripts_line_of_speech_is_the_recording_of_its_line_of_the_spoken_table() {
        let spoken = "\u{feff}Debriefing\t\nATgendebrief1\t{x}\n[END]\t\nMissionMessages\t\n\
            ATstart1\t\"{Mentat : Welcome}\"\n\n;Tutorial\t\nATtutorial1\t{y}\nATstart2\t{x}\n\
            [END]\n";
        let recordings = "ATgendebrief1\tKK-M600E\nATstart2\tKK-M682E\nATstart1\tKK-M680E\n";
        let lines = recorded(spoken, recordings);
        assert_eq!(lines.len(), 3, "only what missions say counts");
        assert_eq!(lines[0].as_deref(), Some("KK-M680"));
        assert_eq!(
            lines[1], None,
            "past a blank line and a comment, a line not recorded"
        );
        assert_eq!(lines[2].as_deref(), Some("KK-M682"));
        let said = written(spoken);
        assert_eq!(
            said[0].as_deref(),
            Some("Mentat : Welcome"),
            "out of its braces"
        );
        assert_eq!(said[1].as_deref(), Some("y"));
        let disc = Path::new("/Volumes/EMPEROR1");
        if !disc.is_dir() {
            return;
        }
        let (lines, _) = spoken_lines(&disc.join(CABINET)).unwrap();
        // The two lines the Atreides' first mission says first, its Mentat's welcome.
        assert_eq!(
            lines[usize::from(642 - FIRST_SPOKEN)].as_deref(),
            Some("KK-M680")
        );
        assert_eq!(
            lines[usize::from(643 - FIRST_SPOKEN)].as_deref(),
            Some("KK-M682")
        );
        // The tutorial's welcome, past the blank lines and comment before it, and the other
        // houses' first missions' welcomes, past the briefings and debriefings of the houses
        // before them.
        for (number, recording) in [(646, "UI-T000"), (901, "YK-M678"), (1098, "RA-M834")] {
            assert_eq!(
                lines[usize::from(number - FIRST_SPOKEN)].as_deref(),
                Some(recording),
                "{number}"
            );
        }
        let house = Path::new("/Volumes/EMPEROR2").join(HOUSE_DIALOG_BAG);
        if let Some(tracks) = music::list(&house) {
            let sounds = Sounds {
                bag: house,
                tracks: tracks
                    .into_iter()
                    .map(|track| (track.name.to_lowercase(), track))
                    .collect(),
                sections: HashMap::new(),
                read: Mutex::default(),
            };
            // Kept as MP3, the welcome lasts seconds, not the blink raw bytes would.
            let line = read_line(&sounds, "kk-m680").unwrap();
            assert!(line.seconds() > 2.0, "{} seconds", line.seconds());
        }
    }
}
