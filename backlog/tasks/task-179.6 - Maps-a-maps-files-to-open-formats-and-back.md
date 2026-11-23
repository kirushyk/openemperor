---
id: TASK-179.6
title: 'Maps: a map''s files to open formats and back'
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-179
ordinal: 357000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A map is a folder in MAPS0001:
- map.inf: text giving WIDTH and HEIGHT in cells.
- test.xbf: the ground's mesh and its start places.
- test.CPT: the wash, as TASK-179.4 describes it.
- test.CPF: 131 072 bytes, 2 a cell of a 256 by 256 grid. battlefield.rs's Terrain::read takes cell types from it.
- texture.dat: as big as test.CPT less 8 bytes; what it holds is to be found (TASK-178.1).
- test.lit: the sun's direction, the ground's ambient and sunlight, and the models' ambient and sunlight.
- thumb.tga and !05%thumb.tga: the radar and preview pictures.
Baseinfo.ini lists the skirmish maps with their ids and players.

What the original's loader counts, by its messages:
- assembly points and spice points ('Map has %d assembly points' and 'Map has %d spice points', 0x4b3ec);
- valley, cliff and AI zones (0x4b1b0);
- entrances ('Map has no entrances defined!', 0xb6670);
- a tile moved to the closest infantry rock (0x37900).
Find where each comes from, test.xbf or test.CPF, and write it down. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory (docs/game-exe.md).

Commands:
- `info MAP`: size, players, start places, spice fields, entrances, and the points and zones above.
- `convert MAP -o DIR`: a folder with the heights as a 16-bit grey PNG, the cell types as a PNG of one colour per value (with a legend), the wash as a PNG, and one text file with map.inf, the light, the start places and the points.
- `convert DIR -o MAP`: back to the original's files, so a map can be changed in an image editor and played through an override folder (TASK-178.5).
Unchanged, the round trip is byte for byte.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 info prints the parts above for every one of the 82 maps
- [ ] #2 Every map converts to the folder above and back, byte for byte when nothing was changed
- [ ] #3 A height or cell type changed in the PNGs shows in the game once the map is converted back into an override folder
- [ ] #4 What test.CPF's values and texture.dat mean, and where start places, entrances, spice and the AI's zones are kept, is written in docs/files.md
<!-- AC:END -->
