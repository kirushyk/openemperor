---
id: TASK-172.2
title: Original's StoreSideBuildings
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-172
ordinal: 343000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ StoreSideBuildings, in GAME.EXE at file offset 0x7fa20, 0xc1 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Records a side's buildings at the end of a battle so a territory played again keeps them ('Map has been played on, but no carryover buildings'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what StoreSideBuildings does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
