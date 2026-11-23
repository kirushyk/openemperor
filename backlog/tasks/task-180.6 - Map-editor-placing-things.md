---
id: TASK-180.6
title: 'Map editor: placing things'
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-180
ordinal: 367000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Open any map, or a copy of one, and place on it:
- start places;
- buildings and units for each side;
- spice fields;
- crates;
- where worms come from;
- the AI's points: entrances, assembly points, and the zones the original's loader counts (0x4b1b0, 0x4b3ec, 0xb6670).

First find where each is kept in the original's files. The start places are in test.xbf (see battlefield.rs, "How many places to start from a map's test.xbf has"); the rest may be test.xbf's dummies or test.CPF's values. TASK-179.6 records what is found.

Placing is snapped to cells, and refused where the game would refuse it.

Save as a map folder in the override (TASK-178.5), listed in the skirmish's maps, and play it from the editor with one key and back. The files are written by TASK-179.6's code. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory (docs/game-exe.md).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each kind of thing above can be placed, moved and removed, and is kept when saved
- [ ] #2 A saved map plays in a skirmish and in the sandbox, with what was placed
- [ ] #3 A map saved without changes is byte for byte the original's
- [ ] #4 Undo and redo
<!-- AC:END -->
