//! The original's sounds on its disc: kept in `AUDIO.BAG`, and named by its sound tables, the
//! `Sfx` files in the cabinet. A table's section lists the sounds one of which plays for
//! something: `[ATKindjalSelection]` lists what a Kindjal says when it is selected, and
//! `[ATGroupSelection]` what a group of Atreides says.

use crate::archive;
use crate::game::{Faction, Said, Speech};
use crate::music::{self, Track};
use crate::units::{self, Role};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

const CABINET: &str = "INSTALL/Game1.cab";
const BAG: &str = "INSTALL/DATA/SFX/AUDIO.BAG";
/// The bag of what the battlefield says to the player, on the disc that installs the game.
const DIALOG_BAG: &str = "INSTALL/DATA/DIALOG/DIALOG.BAG";
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
    }
}

/// A sound to play once: mono or stereo samples, interleaved.
pub struct Line {
    pub samples: Vec<f32>,
    pub channels: u32,
    pub rate: u32,
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
}
