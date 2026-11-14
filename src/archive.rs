//! The game's packed files on its disc: the install cabinet, and the data files whose entries an
//! index names, each index kept in the cabinet and its data loose beside it.

use crate::inflate;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const CABINET_MAGIC: &[u8; 4] = b"MSCF";
/// A cabinet's head before its optional parts, and the flag saying it has room set aside.
const CABINET_HEAD: usize = 36;
const RESERVES_ROOM: u16 = 4;
const CHAINED: [u16; 2] = [1, 2];
/// How a cabinet's folder is packed: not at all, or in MSZIP blocks.
const STORED: u16 = 0;
const MSZIP: u16 = 1;

fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

/// The bytes after `at` up to the next zero byte, and where that zero is.
fn string_at(data: &[u8], at: usize) -> Option<(&[u8], usize)> {
    let end = at + data.get(at..)?.iter().position(|&byte| byte == 0)?;
    Some((&data[at..end], end))
}

/// A file out of the Microsoft cabinet at `path`, by its name in there, any case.
pub fn from_cabinet(path: &Path, name: &str) -> Option<Vec<u8>> {
    file_in_cabinet(&std::fs::read(path).ok()?, name)
}

fn file_in_cabinet(cabinet: &[u8], name: &str) -> Option<Vec<u8>> {
    if cabinet.get(..4)? != CABINET_MAGIC {
        return None;
    }
    let files_at = u32_at(cabinet, 16)? as usize;
    let (folders, files, flags) = (
        u16_at(cabinet, 26)?,
        u16_at(cabinet, 28)?,
        u16_at(cabinet, 30)?,
    );
    let mut at = CABINET_HEAD;
    let (mut folder_room, mut block_room) = (0, 0);
    if flags & RESERVES_ROOM != 0 {
        let head_room = usize::from(u16_at(cabinet, at)?);
        (folder_room, block_room) = (usize::from(cabinet[at + 2]), usize::from(cabinet[at + 3]));
        at += 4 + head_room;
    }
    // The names of the cabinets before and after this one, if it is one of a set.
    for flag in CHAINED {
        if flags & flag != 0 {
            for _ in 0..2 {
                at = string_at(cabinet, at)?.1 + 1;
            }
        }
    }
    let folders_at = at;
    let mut at = files_at;
    for _ in 0..files {
        let size = u32_at(cabinet, at)? as usize;
        let offset = u32_at(cabinet, at + 4)? as usize;
        let folder = usize::from(u16_at(cabinet, at + 8)?);
        let (file_name, end) = string_at(cabinet, at + 16)?;
        at = end + 1;
        if !file_name.eq_ignore_ascii_case(name.as_bytes()) || folder >= usize::from(folders) {
            continue;
        }
        let folder_at = folders_at + folder * (8 + folder_room);
        let data_at = u32_at(cabinet, folder_at)? as usize;
        let blocks = u16_at(cabinet, folder_at + 4)?;
        let packing = u16_at(cabinet, folder_at + 6)? & 0x0f;
        let unpacked = unpack_folder(cabinet, data_at, blocks, block_room, packing, offset + size)?;
        return unpacked.get(offset..offset + size).map(<[u8]>::to_vec);
    }
    None
}

/// A folder's blocks unpacked one after another, until there are `wanted` bytes of it.
fn unpack_folder(
    cabinet: &[u8],
    mut at: usize,
    blocks: u16,
    block_room: usize,
    packing: u16,
    wanted: usize,
) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(wanted);
    for _ in 0..blocks {
        if out.len() >= wanted {
            break;
        }
        let packed = usize::from(u16_at(cabinet, at + 4)?);
        let start = at + 8 + block_room;
        let data = cabinet.get(start..start + packed)?;
        match packing {
            STORED => out.extend_from_slice(data),
            // Each block starts "CK", and copies in it may reach back into the ones before.
            MSZIP if data.starts_with(b"CK") => {
                inflate::inflate(&data[2..], &mut out)?;
            }
            _ => return None,
        }
        at = start + packed;
    }
    Some(out)
}

/// One of the game's data files and its index: what each entry is called, any case, and where
/// it lies in the data.
pub struct Archive {
    data: File,
    entries: HashMap<String, Entry>,
}

#[derive(Clone, Copy, Debug)]
struct Entry {
    packed: bool,
    size: u32,
    offset: u32,
}

/// The flag in an index entry saying it is packed with zlib, behind its unpacked size.
const ZLIB: u32 = 2;
/// An index entry before its name: the name's length, a time, flags, the size in the data,
/// the size unpacked, and where in the data it starts.
const ENTRY_HEAD: usize = 24;

impl Archive {
    pub fn open(index: &[u8], data: &Path) -> Option<Self> {
        Some(Archive {
            data: File::open(data).ok()?,
            entries: entries(index)?,
        })
    }

    /// The entry of this name, any case, unpacked.
    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        let entry = *self.entries.get(&name.to_lowercase())?;
        let mut data = &self.data;
        data.seek(SeekFrom::Start(u64::from(entry.offset))).ok()?;
        let mut bytes = vec![0; entry.size as usize];
        data.read_exact(&mut bytes).ok()?;
        entry.unpack(bytes)
    }

    /// The names of its entries, in lower case.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
}

/// The entry of this name, any case, out of a data file already read, with its index.
pub fn read_in(index: &[u8], data: &[u8], name: &str) -> Option<Vec<u8>> {
    let entry = *entries(index)?.get(&name.to_lowercase())?;
    let start = entry.offset as usize;
    entry.unpack(data.get(start..start + entry.size as usize)?.to_vec())
}

/// What an index names, each by its name in lower case.
fn entries(index: &[u8]) -> Option<HashMap<String, Entry>> {
    let mut entries = HashMap::new();
    let mut at = 0;
    while at + ENTRY_HEAD <= index.len() {
        let field = |n: usize| u32_at(index, at + n * 4);
        let length = field(0)? as usize;
        let entry = Entry {
            packed: field(2)? & ZLIB != 0,
            size: field(3)?,
            offset: field(5)?,
        };
        let name = index.get(at + ENTRY_HEAD..at + ENTRY_HEAD + length)?;
        let name = name.split(|&byte| byte == 0).next().unwrap_or(name);
        entries.insert(String::from_utf8_lossy(name).to_lowercase(), entry);
        at += ENTRY_HEAD + length;
    }
    Some(entries)
}

impl Entry {
    fn unpack(self, bytes: Vec<u8>) -> Option<Vec<u8>> {
        if self.packed {
            inflate::unzlib(bytes.get(4..)?)
        } else {
            Some(bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cabinet of one folder of one stored block, holding the files given.
    fn cabinet(files: &[(&str, &[u8])]) -> Vec<u8> {
        let data: Vec<u8> = files.iter().flat_map(|(_, bytes)| bytes.to_vec()).collect();
        let mut table = Vec::new();
        let mut offset = 0;
        for (name, bytes) in files {
            table.extend((bytes.len() as u32).to_le_bytes());
            table.extend((offset as u32).to_le_bytes());
            table.extend([0; 8]);
            table.extend(name.as_bytes());
            table.push(0);
            offset += bytes.len();
        }
        let folders_at = CABINET_HEAD;
        let files_at = folders_at + 8;
        let block_at = files_at + table.len();
        let mut cabinet = CABINET_MAGIC.to_vec();
        cabinet.extend([0; 12]);
        cabinet.extend((files_at as u32).to_le_bytes());
        cabinet.extend([0; 6]);
        cabinet.extend(1u16.to_le_bytes());
        cabinet.extend((files.len() as u16).to_le_bytes());
        cabinet.extend([0; 6]);
        cabinet.extend((block_at as u32).to_le_bytes());
        cabinet.extend(1u16.to_le_bytes());
        cabinet.extend(STORED.to_le_bytes());
        cabinet.extend(table);
        cabinet.extend([0; 4]);
        cabinet.extend((data.len() as u16).to_le_bytes());
        cabinet.extend((data.len() as u16).to_le_bytes());
        cabinet.extend(data);
        cabinet
    }

    #[test]
    fn a_file_comes_out_of_a_cabinet_by_its_name_in_any_case() {
        let packed = cabinet(&[
            ("\\data\\one.rfh", b"first"),
            ("\\data\\two.rfh", b"second"),
        ]);
        assert_eq!(
            file_in_cabinet(&packed, "\\DATA\\TWO.RFH").as_deref(),
            Some(&b"second"[..])
        );
        assert_eq!(file_in_cabinet(&packed, "\\data\\three.rfh"), None);
    }
}
