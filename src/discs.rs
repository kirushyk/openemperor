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

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
