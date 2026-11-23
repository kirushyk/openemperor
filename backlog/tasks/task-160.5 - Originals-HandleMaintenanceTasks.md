---
id: TASK-160.5
title: Original's HandleMaintenanceTasks
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-160
ordinal: 327000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ HandleMaintenanceTasks, in GAME.EXE at file offset 0x69800, 0x56 bytes; file offset 0x69856, 0xda bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Sends an ornithopter that needs to rearm to one of its side's helipads ('Ornithopter - Need to rearm', 'Sending to pad'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what HandleMaintenanceTasks does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
