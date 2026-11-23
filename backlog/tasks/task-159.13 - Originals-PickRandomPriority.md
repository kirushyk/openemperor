---
id: TASK-159.13
title: Original's PickRandomPriority
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 308000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ PickRandomPriority, in GAME.EXE at file offset 0x497f0, 0xa0 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Draws a script priority: Random mod 100 falls in priority 1 for 50, 2 for 30, 3 for 15, 4 for 5 ('An AI script has a priority of %d, max implemented is 4'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what PickRandomPriority does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
