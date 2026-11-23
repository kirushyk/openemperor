---
id: TASK-159.18
title: Original's FindTacticAssemblyPoint
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 313000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ FindTacticAssemblyPoint, in GAME.EXE at file offset 0x2b380, 0x245 bytes; file offset 0x2b6a0, 0x1a0 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Finds where a tactic's team gathers: at a ramp of its rock towards the enemy ('No ramps on rock'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what FindTacticAssemblyPoint does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
