---
id: TASK-178.5
title: Loose files take the place of archived ones
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-178
ordinal: 350000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Let a file in a folder take the place of the archive entry of the same name, so that the game's data can be changed without repacking anything. For example, <override>/MODEL0001/Rules.txt would be read instead of Rules.txt in MODEL0001.RFD. So would the same file under its resource.cfg folder, data/model/Rules.txt.

The original's install did much the same:
- Its resource.cfg points each kind of file at a folder: data\model, data\maps, data\missions and so on.
- Its resource library has code that walks a folder: 0x3110, 0x193 bytes, FindFirstFileA (import 0x5c4150) on '%s\%s' and '%s\%s\%s'. That suggests it reads loose files before its libraries.

Confirm from 0x3110 and the library's lookup which wins and in what order, and write it in docs/game-exe.md. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory (docs/game-exe.md).

Where overrides come from: any `--data` folder that is not a disc, and an `override` folder beside the options file (TASK-163).

A new map folder dropped in shows in the skirmish list only through Baseinfo.ini, which is what the original lists from. Decide whether an override Baseinfo.ini is needed, or whether map folders are also found by themselves; say which, and why, in the decisions doc.

The tool's `extract` (TASK-179) writes this layout, so unpack, edit, run works without packing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A file in an override folder is read in place of the archive entry of the same name, in any archive
- [ ] #2 Rules.txt changed in an override folder changes the game
- [ ] #3 A folder that the tool's extract wrote works as an override folder as it is
- [ ] #4 What the original does with loose files is written down from its code in docs/game-exe.md
- [ ] #5 Tests cover an override replacing an entry and adding a new one
<!-- AC:END -->
