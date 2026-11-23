---
id: TASK-159.21
title: Original's LocationManager and GetLocation
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 316000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ LocationManager and GetLocation, in GAME.EXE at file offset 0x33230, 0x90 bytes; file offset 0x330e0, 0x50 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Keeps the AI's named locations (targets, zones, points; 'allocated %d location slots') and returns one by id ('Illegal location id of %d'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what LocationManager and GetLocation does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
