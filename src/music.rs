//! The music on the game's disc: tracks packed one after another into a `.bag` file, behind an
//! index that starts with "GABA".

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::thread;

/// Only tracks lasting longer than this, in seconds, play in battle, which leaves out the short
/// pieces for the menus and the score.
pub const LEAST_BATTLE_SECONDS: f32 = 60.0;

const MAGIC: &[u8; 4] = b"GABA";
/// The index's head: the magic, its version, how many tracks, and how long each entry is.
const HEAD: usize = 16;
/// How much of an entry its name takes, and where its offset and size follow.
const NAME: usize = 32;
const LEAST_ENTRY: usize = NAME + 8;
/// More tracks than this means it is not an index at all.
const MOST_TRACKS: usize = 4096;

/// Where one track lies in the bag, in bytes from its start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub name: String,
    pub offset: u32,
    pub size: u32,
}

impl Track {
    /// Where FFmpeg finds this track inside the bag at `bag`.
    pub fn location(&self, bag: &Path) -> String {
        let end = u64::from(self.offset) + u64::from(self.size);
        format!(
            "subfile,,start,{},end,{end},,:{}",
            self.offset,
            bag.display()
        )
    }
}

/// The track of this name in the bag at `path`, if it is there and is a bag.
pub fn find(path: &Path, name: &str) -> Option<Track> {
    list(path)?.into_iter().find(|track| track.name == name)
}

/// Every track in the bag at `path`, if it is there and is a bag.
fn list(path: &Path) -> Option<Vec<Track>> {
    let mut file = File::open(path).ok()?;
    let mut head = [0; HEAD];
    file.read_exact(&mut head).ok()?;
    let (count, entry) = size_of_index(&head)?;
    let mut index = head.to_vec();
    index.resize(HEAD + count * entry, 0);
    file.read_exact(&mut index[HEAD..]).ok()?;
    tracks(&index)
}

/// How long decoded sound lasts, in seconds.
pub fn seconds(sound: &[f32]) -> f32 {
    sound.len() as f32 / (crate::video::SOUND_RATE * crate::video::SOUND_CHANNELS) as f32
}

/// The battle music of the bag at `path`: its tracks longer than `LEAST_BATTLE_SECONDS`, in
/// the bag's order and round again, each decoded on a thread of its own before it is wanted.
/// Nothing comes if no track is long enough.
pub fn battle_tracks(path: &Path) -> Receiver<Vec<f32>> {
    let tracks = list(path).unwrap_or_default();
    let path = path.to_owned();
    round_robin(
        tracks,
        move |track| crate::video::decode_sound(&track.location(&path)),
        |sound| seconds(sound) > LEAST_BATTLE_SECONDS,
    )
}

/// Decodes the tracks in turn, over and over, and sends on those that `long` keeps. A track
/// it leaves out once is not decoded again, and the thread ends when none is kept or nobody
/// is receiving.
fn round_robin<S: Send + 'static>(
    tracks: Vec<Track>,
    decode: impl Fn(&Track) -> S + Send + 'static,
    long: impl Fn(&S) -> bool + Send + 'static,
) -> Receiver<S> {
    // Sending waits until the track before is taken, so at most one is held decoded ahead.
    let (send, receive) = mpsc::sync_channel(0);
    thread::spawn(move || {
        let mut kept = vec![true; tracks.len()];
        while kept.contains(&true) {
            for (track, kept) in tracks.iter().zip(&mut kept) {
                if !*kept {
                    continue;
                }
                let sound = decode(track);
                *kept = long(&sound);
                if *kept && send.send(sound).is_err() {
                    return;
                }
            }
        }
    });
    receive
}

/// How many tracks the index holds and how long each entry is, from its head.
fn size_of_index(head: &[u8]) -> Option<(usize, usize)> {
    if head.get(..4)? != MAGIC {
        return None;
    }
    let count = word(head, 8)? as usize;
    let entry = word(head, 12)? as usize;
    (count <= MOST_TRACKS && entry >= LEAST_ENTRY).then_some((count, entry))
}

/// Every track an index lists.
fn tracks(index: &[u8]) -> Option<Vec<Track>> {
    let (count, entry) = size_of_index(index)?;
    (0..count)
        .map(|i| {
            let at = HEAD + i * entry;
            let name = index.get(at..at + NAME)?;
            let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(NAME)];
            Some(Track {
                name: String::from_utf8_lossy(name).into_owned(),
                offset: word(index, at + NAME)?,
                size: word(index, at + NAME + 4)?,
            })
        })
        .collect()
}

fn word(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, offset: u32, size: u32) -> Vec<u8> {
        let mut bytes = vec![0; 64];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        bytes[32..36].copy_from_slice(&offset.to_le_bytes());
        bytes[36..40].copy_from_slice(&size.to_le_bytes());
        bytes
    }

    fn index(entries: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        for number in [4, entries.len() as u32, 64] {
            bytes.extend(number.to_le_bytes());
        }
        bytes.extend(entries.concat());
        bytes
    }

    #[test]
    fn a_bag_lists_its_tracks_by_name_with_where_they_lie() {
        let bag = index(&[
            entry("IN_Menu", 528, 512_000),
            entry("IN_Score", 512_528, 9),
        ]);
        let found = tracks(&bag).unwrap();
        assert_eq!(
            found,
            [
                Track {
                    name: "IN_Menu".into(),
                    offset: 528,
                    size: 512_000
                },
                Track {
                    name: "IN_Score".into(),
                    offset: 512_528,
                    size: 9
                },
            ]
        );
        assert_eq!(
            found[0].location(Path::new("/disc/music.bag")),
            "subfile,,start,528,end,512528,,:/disc/music.bag"
        );
    }

    #[test]
    fn the_menus_music_on_the_disc_decodes_when_it_is_mounted() {
        let bag = Path::new("/Volumes/EMPEROR1/INSTALL/DATA/MUSIC/music.bag");
        if !bag.exists() {
            return;
        }
        let track = find(bag, "IN_Menu").unwrap();
        let sound = crate::video::decode_sound(&track.location(bag));
        let seconds = sound.len() as f32 / (crate::video::SOUND_RATE * 2) as f32;
        assert!((seconds - 32.0).abs() < 0.5, "{seconds} seconds");
    }

    #[test]
    fn battle_music_plays_only_the_long_tracks_round_and_round() {
        let lengths = [
            ("IN_Menu", 32.0),
            ("War", 274.0),
            ("Sting", 60.0),
            ("Worm", 61.0),
        ];
        let tracks = lengths
            .iter()
            .map(|&(name, _)| Track {
                name: name.into(),
                offset: 0,
                size: 0,
            })
            .collect();
        let played = round_robin(
            tracks,
            move |track| {
                lengths
                    .iter()
                    .find(|(name, _)| *name == track.name)
                    .copied()
            },
            |found| found.is_some_and(|(_, seconds)| seconds > LEAST_BATTLE_SECONDS),
        );
        let names: Vec<_> = played
            .iter()
            .take(5)
            .map(|found| found.unwrap().0)
            .collect();
        assert_eq!(names, ["War", "Worm", "War", "Worm", "War"]);
    }

    #[test]
    fn battle_music_ends_when_no_track_is_long_enough() {
        let short = Track {
            name: "IN_Menu".into(),
            offset: 0,
            size: 0,
        };
        let played = round_robin(vec![short], |_| 32.0, |&s| s > LEAST_BATTLE_SECONDS);
        assert!(played.recv().is_err());
    }

    #[test]
    fn what_is_not_a_bag_or_is_cut_short_lists_nothing() {
        assert_eq!(tracks(b"RIFF....WAVEfmt "), None);
        let mut bag = index(&[entry("IN_Menu", 528, 512_000)]);
        bag.truncate(40);
        assert_eq!(tracks(&bag), None);
    }
}
