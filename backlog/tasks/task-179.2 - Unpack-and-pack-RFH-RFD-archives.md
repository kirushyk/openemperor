---
id: TASK-179.2
title: Unpack and pack RFH/RFD archives
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-179
ordinal: 353000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The format, as archive.rs reads it:
- The index (RFH) is entries one after another. Each starts with six little-endian u32s: the name's length including its NUL; a time; flags, where 2 means zlib; the size in the data; the unpacked size; and where it starts in the data.
- Then comes the name, with \ between folders.
- A zlib entry's data starts with its unpacked size as a u32, then a zlib stream.
- The data (RFD) is the entries' bytes.
- The indexes of all but 3DDATA0002 and the house discs' MOVIES0001 are kept in Game1.cab.

Commands:
- `extract` unpacks an archive, through the frame.
- `pack DIR -o NAME` writes NAME.RFH and NAME.RFD from a folder.
- `--store` keeps every entry unpacked.
- By default, pack the way the original does: it zlibs most entries but stores some, such as ai_atreides.ini, every movie, and a few small files.
- `--like ARCHIVE` takes the order, times and flags from an existing archive.

Packing needs a deflate compressor; inflate.rs only inflates. Add one: stored blocks first, which are already valid zlib, then fixed and dynamic Huffman codes with a simple hash-chain matcher. PNG (TASK-179.4) and the cabinet (TASK-179.3) use it too.

Find out what the time field is (DOS date and time, or Unix seconds) and write it in docs/files.md.

An archive we pack should load in the original too. Its opener (0x3ab0, 0x110 bytes) and its streamer (0x3d4b, 0x225 bytes) show which fields it reads; write them down. That can only be checked by hand, on Windows or under Wine. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory (docs/game-exe.md).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extract unpacks every entry of every archive on the discs, folder paths kept
- [ ] #2 pack makes an RFH/RFD that our game reads as it reads the original's
- [ ] #3 extract then `pack --like` gives back the same index and the same entries, stored ones byte for byte
- [ ] #4 zlib entries we pack inflate to the same bytes, with our inflate and with zlib's (a checked-in fixture)
- [ ] #5 A deflate compressor beside inflate.rs, tested on its own
- [ ] #6 What the time field holds, and the fields the original's opener reads, are written in docs/files.md
<!-- AC:END -->
