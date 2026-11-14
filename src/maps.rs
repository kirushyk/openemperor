//! The original's maps on its disc: which of them a skirmish is fought on, as its `Baseinfo.ini`
//! lists them with how many players each takes, what each is called, and the files each is
//! made of, in its own folder of the maps' data file. The campaign's maps may be listed after
//! them.

use crate::archive::{self, Archive};
use crate::battlefield;
use std::path::Path;
use std::sync::OnceLock;

const CABINET: &str = "INSTALL/Game1.cab";
const INDEX: &str = "\\data\\MAPS0001.RFH";
const DATA: &str = "INSTALL/DATA/MAPS0001.RFD";
/// The index and data of the game's text, both kept in the cabinet, and the text in them
/// that names the maps.
const TEXT_INDEX: &str = "\\data\\STRINGS0001.RFH";
const TEXT_DATA: &str = "\\data\\STRINGS0001.RFD";
const TEXT: &str = "E_Output_Text_Pickup.txt";
const LIST: &str = "Baseinfo.ini";
/// What the ids of the skirmish maps start with, and the file each map's ground is in.
const SKIRMISH: &str = "#m";
const MAP_FILE: &str = "test.xbf";

/// One map a skirmish can be fought on.
pub struct Map {
    /// What the original shows for it.
    pub name: String,
    /// How many sides it takes at most.
    pub players: usize,
    /// Whether it is one of the campaign's, not made for a skirmish.
    pub campaign: bool,
    /// Its folder in the maps' data file.
    folder: String,
}

struct Maps {
    /// The maps' data file; tests have none.
    archive: Option<Archive>,
    list: Vec<Map>,
}

static MAPS: OnceLock<Maps> = OnceLock::new();

/// The maps the disc has, once read. Tests have maps of their own, all blank sand.
fn maps() -> Option<&'static Maps> {
    #[cfg(test)]
    return Some(MAPS.get_or_init(|| {
        Maps {
            archive: None,
            list: (1..=6)
                .map(|n| Map {
                    name: format!("Test Map {n}"),
                    players: 8,
                    campaign: false,
                    folder: String::new(),
                })
                .collect(),
        }
    }));
    #[cfg(not(test))]
    MAPS.get()
}

/// Reads which maps the disc mounted at `disc` has, the campaign's too if `campaign`. Without
/// it, there are none.
pub fn load(disc: &Path, campaign: bool) {
    if let Some(maps) = read_maps(disc, campaign) {
        let _ = MAPS.set(maps);
    }
}

/// The maps a skirmish can be fought on, in the order the original lists them.
pub fn list() -> &'static [Map] {
    maps().map_or(&[], |maps| &maps.list)
}

/// One of a map's files, by its name in the map's folder.
pub fn file(map: usize, name: &str) -> Option<Vec<u8>> {
    let maps = maps()?;
    let folder = &maps.list.get(map)?.folder;
    maps.archive.as_ref()?.read(&format!("{folder}\\{name}"))
}

fn read_maps(disc: &Path, campaign: bool) -> Option<Maps> {
    let cabinet = disc.join(CABINET);
    let index = archive::from_cabinet(&cabinet, INDEX)?;
    let archive = Archive::open(&index, &disc.join(DATA))?;
    let names = archive::from_cabinet(&cabinet, TEXT_INDEX)
        .zip(archive::from_cabinet(&cabinet, TEXT_DATA))
        .and_then(|(index, data)| archive::read_in(&index, &data, TEXT))
        .map(|text| utf16(&text))
        .unwrap_or_default();
    let folders: Vec<String> = archive
        .names()
        .filter_map(|name| name.split_once('\\').map(|(folder, _)| folder.to_string()))
        .collect();
    let listed = String::from_utf8_lossy(&archive.read(LIST)?).into_owned();
    let mut list: Vec<Map> = listed
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let id = fields.next().filter(|id| id.starts_with('#'))?;
            let players = fields.next()?.parse().ok()?;
            let folder = folders.iter().find(|folder| is_folder_of(folder, id))?;
            Some(Map {
                name: name_of(&names, id).unwrap_or_else(|| id.to_string()),
                players,
                campaign: false,
                folder: folder.clone(),
            })
        })
        .collect();
    if campaign {
        let mut others: Vec<&String> = folders
            .iter()
            .filter(|folder| !folder.starts_with(SKIRMISH))
            .filter(|folder| list.iter().all(|map| &map.folder != *folder))
            .collect();
        others.sort();
        others.dedup();
        others.sort_by_key(|folder| id_order(folder));
        for folder in others {
            let players = archive
                .read(&format!("{folder}\\{MAP_FILE}"))
                .map_or(0, |file| battlefield::starts_in(&file));
            if players > 0 {
                let id = folder.split_whitespace().next().unwrap_or(folder);
                list.push(Map {
                    name: name_of(&names, &id.to_uppercase()).unwrap_or_else(|| title(folder)),
                    players,
                    campaign: true,
                    folder: folder.clone(),
                });
            }
        }
    }
    Some(Maps {
        archive: Some(archive),
        list,
    })
}

/// Where a map's folder goes among the others: by the letter of its id, then its number.
fn id_order(folder: &str) -> (String, u32) {
    let id = folder.split_whitespace().next().unwrap_or(folder);
    let digits = id.trim_start_matches(|c: char| !c.is_ascii_digit());
    let number = digits
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap_or("");
    (
        id[..id.len() - digits.len()].to_string(),
        number.parse().unwrap_or(0),
    )
}

/// A name for a map the game's text does not name, out of its folder's: its words past its
/// id, without its maker's initials in front or how it was made after, each capitalized.
fn title(folder: &str) -> String {
    const MAKERS: [&str; 2] = ["jf", "gm"];
    const MADE: [&str; 3] = ["s", "m", "lod2"];
    let mut words: Vec<&str> = folder.split_whitespace().skip(1).collect();
    if words.first().is_some_and(|word| MAKERS.contains(word)) {
        words.remove(0);
    }
    while words.last().is_some_and(|word| MADE.contains(word)) {
        words.pop();
    }
    words
        .iter()
        .map(|word| {
            let mut letters = word.chars();
            letters.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(letters).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether a folder is the map of this id: its name starts with the id, and goes on with
/// anything but another digit.
fn is_folder_of(folder: &str, id: &str) -> bool {
    folder.len() > id.len()
        && folder[..id.len()].eq_ignore_ascii_case(id)
        && !folder.as_bytes()[id.len()].is_ascii_digit()
}

/// What the game's text calls the map of this id: the words in braces on its line.
fn name_of(text: &str, id: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (key, rest) = line.split_once('\t')?;
        let name = rest.trim_start().strip_prefix('{')?.split_once('}')?.0;
        (key == id).then(|| name.trim().to_string())
    })
}

/// Text written as UTF-16 with its low bytes first, as the game's text files are.
fn utf16(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16_lossy(&units)
        .trim_start_matches('\u{feff}')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_map_is_found_in_the_folder_its_id_starts_but_not_in_one_of_a_longer_id() {
        assert!(is_folder_of("#m29 gm fishes plain s 8", "#M29"));
        assert!(is_folder_of("#m12_jf_s_2-4p lod2", "#M12"));
        assert!(!is_folder_of(
            "#m11 jf ruins of calea berm s 2-4 lod2",
            "#M1"
        ));
        assert!(!is_folder_of("#m1", "#M1"));
    }

    #[test]
    fn a_map_is_called_what_its_line_of_text_says_in_braces() {
        let text = "#M1\t{Koth Crater }\t\t\n#M29\t\t{Fishers Plain }\n";
        assert_eq!(name_of(text, "#M29").as_deref(), Some("Fishers Plain"));
        assert_eq!(name_of(text, "#M1").as_deref(), Some("Koth Crater"));
        assert_eq!(name_of(text, "#M2"), None);
    }

    #[test]
    fn a_map_the_text_does_not_name_is_called_by_its_folder_and_comes_in_the_order_of_its_id() {
        assert_eq!(
            title("#t9  jf arrakeen spice field s lod2"),
            "Arrakeen Spice Field"
        );
        assert_eq!(
            title("#d1 atreides homeworld defence lod2"),
            "Atreides Homeworld Defence"
        );
        assert!(id_order("#t9  jf a") < id_order("#t10 jf b"));
        assert!(id_order("#a3 x") < id_order("#t1 y"));
    }

    #[test]
    fn text_comes_out_of_utf16_without_its_mark() {
        assert_eq!(utf16(&[0xff, 0xfe, b'#', 0, b'M', 0]), "#M");
    }
}
