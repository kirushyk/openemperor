---
id: TASK-159.4
title: Original's CAiObjectKnowledge
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 299000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CAiObjectKnowledge, in GAME.EXE at file offset 0x3ae50, 0xa4 bytes; file offset 0x3aef4, 0x29e bytes; file offset 0x3b192, 0x10e bytes; file offset 0x3c160, 0x30 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: What the AI knows of each kind: the indexes of each house's refinery, barracks, refinery pad, small windtrap, wall, scout and turret, the superweapons, FRADVFremen and the harvester, looked up by name ('Could not get attrib for %s'), and each kind's threat value. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CAiObjectKnowledge does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
