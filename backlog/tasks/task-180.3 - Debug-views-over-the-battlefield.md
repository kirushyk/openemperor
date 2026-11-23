---
id: TASK-180.3
title: Debug views over the battlefield
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-180
ordinal: 364000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Views turned on and off one by one, drawn over the battle, with a side pane of text like the original's debug pane (0xa1b30). The text uses our font, or the original's bitmapfont.tga with its font maps.

The pane shows:
- frames a second, frame time, tick, game speed and seed;
- for the cell under the cursor: coordinates, height, its CPF cell type and what can cross it, its zone, spice, and which sides see it (the fog's masks);
- for the selected object, its fields: side, kind, position, health of max, orders queued, target, veterancy, ammunition, and the AI's tactic or pattern for it;
- for each side: cash, power, and the AI's state, meaning its money split, base plan and threat values (TASK-158 to TASK-160).

The layers drawn on the ground:
- passability by kind of unit;
- slopes and cliffs;
- build areas, the eight islands;
- the AI's zones, entrances and assembly points;
- spice fields;
- paths;
- weapon ranges and sight;
- bounding boxes, which the original's art scripts give as <<BoundingBox (0x83b50);
- wireframe. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory (docs/game-exe.md).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each view above can be turned on and off by a key or a console command, in geek mode only
- [ ] #2 The pane's figures are those the simulation uses (tests check a few against units.rs's state)
- [ ] #3 Turning views on costs no change to the simulation: a battle with them on plays out tick for tick as one without
<!-- AC:END -->
