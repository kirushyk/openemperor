---
id: TASK-158.6
title: Original's ConnectRoadsToRamp
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 286000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ ConnectRoadsToRamp, in GAME.EXE at file offset 0x36ae1, 0x29f bytes; file offset 0x36d80, 0x170 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Joins the base's roads to a ramp off the rock, trying a second road if the first fails ('Side %d, Initial road met ramp, dir: %d %d', 'Second road NOT met, trying other', 'Cannot connect roads to ramp as no road in default position'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what ConnectRoadsToRamp does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
