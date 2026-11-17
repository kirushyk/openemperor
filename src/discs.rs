//! The game's four discs, whichever of them are mounted: the first installs the game and has
//! what every house shares; the second, third and fourth are a house's own, with its movies,
//! music and briefings.

use std::path::{Path, PathBuf};

/// Where a disc of this number shows once it is mounted.
const VOLUMES: &str = "/Volumes/EMPEROR";
/// How many discs the game came on.
const DISCS: u32 = 4;
/// What only the disc that installs the game has.
const INSTALL: &str = "INSTALL";
/// The first disc's bag of music, and a house disc's.
const SHARED_MUSIC: &str = "INSTALL/DATA/MUSIC/music.bag";
const HOUSE_MUSIC: &str = "MUSIC.BAG";
/// A house disc's archive of movies: its index, and the movies themselves.
const MOVIES_INDEX: &str = "MOVIES0001.RFH";
const MOVIES_DATA: &str = "MOVIES0001.RFD";
/// How long the head of an entry of an archive's index is, before its name: six numbers.
const ENTRY_HEAD: usize = 24;

/// The discs that are mounted, in the order of their numbers.
pub fn mounted() -> Vec<PathBuf> {
    (1..=DISCS)
        .map(|number| PathBuf::from(format!("{VOLUMES}{number}")))
        .filter(|disc| disc.is_dir())
        .collect()
}

/// Every bag of music on these discs.
pub fn music_bags(discs: &[PathBuf]) -> Vec<PathBuf> {
    discs
        .iter()
        .flat_map(|disc| [disc.join(SHARED_MUSIC), disc.join(HOUSE_MUSIC)])
        .filter(|bag| bag.is_file())
        .collect()
}

/// Whether a disc is the one the game installs from.
pub fn installs(disc: &Path) -> bool {
    disc.join(INSTALL).is_dir()
}

/// Where FFmpeg finds the movie of this name in a house disc's archive of movies, if one of
/// these discs has it: the archive keeps its movies whole, one after another.
pub fn movie(discs: &[PathBuf], name: &str) -> Option<String> {
    discs.iter().find_map(|disc| {
        let index = std::fs::read(disc.join(MOVIES_INDEX)).ok()?;
        let (offset, size) = find_in_index(&index, name)?;
        let data = disc.join(MOVIES_DATA);
        Some(format!(
            "subfile,,start,{offset},end,{},,:{}",
            offset + size,
            data.display()
        ))
    })
}

/// Where the file of this name lies in an archive, by its index: how far in, and how long.
/// Each entry is six numbers, the first the length of the name that follows them, the fourth
/// the file's size and the sixth where it starts.
fn find_in_index(index: &[u8], name: &str) -> Option<(u64, u64)> {
    let mut at = 0;
    while at + ENTRY_HEAD <= index.len() {
        let field = |n: usize| {
            let bytes = index.get(at + n * 4..at + n * 4 + 4)?;
            Some(u32::from_le_bytes(bytes.try_into().ok()?))
        };
        let length = field(0)? as usize;
        let named = index.get(at + ENTRY_HEAD..at + ENTRY_HEAD + length)?;
        let end = named.iter().position(|&b| b == 0).unwrap_or(length);
        if named[..end].eq_ignore_ascii_case(name.as_bytes()) {
            return Some((u64::from(field(5)?), u64::from(field(3)?)));
        }
        at += ENTRY_HEAD + length;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_movie_is_found_in_an_archives_index_by_its_name_whatever_its_case() {
        let entry = |name: &str, size: u32, offset: u32| {
            let mut bytes = Vec::new();
            for field in [name.len() as u32 + 1, 0, 0, size, size, offset] {
                bytes.extend(field.to_le_bytes());
            }
            bytes.extend(name.as_bytes());
            bytes.push(0);
            bytes
        };
        let index = [
            entry("A00_F00E.BIK", 100, 0),
            entry("A01_F00E.BIK", 50, 100),
        ]
        .concat();
        assert_eq!(find_in_index(&index, "a01_f00e.bik"), Some((100, 50)));
        assert_eq!(find_in_index(&index, "A00_F00E.BIK"), Some((0, 100)));
        assert_eq!(find_in_index(&index, "A02_F00E.BIK"), None);
        assert_eq!(find_in_index(&index[..30], "A01_F00E.BIK"), None);
    }

    #[test]
    fn the_mounted_discs_are_found_and_the_first_is_the_one_that_installs() {
        let discs = mounted();
        assert!(discs.len() <= DISCS as usize);
        for bag in music_bags(&discs) {
            assert!(crate::music::list(&bag).is_some(), "{}", bag.display());
        }
        if let Some(first) = discs.first().filter(|disc| disc.ends_with("EMPEROR1")) {
            assert!(installs(first));
        }
        // The Atreides disc has their first movie, and it plays from inside its archive.
        if discs.iter().any(|disc| disc.ends_with("EMPEROR2")) {
            let found = movie(&discs, "A00_F00E.BIK").expect("on the disc");
            let opened = crate::video::Movie::open(Path::new(&found)).expect("it opens");
            assert!(opened.width > 0 && opened.height > 0);
        }
    }
}
