---
id: TASK-159.14
title: Original's GetScriptIndexByPriority
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 309000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ GetScriptIndexByPriority, in GAME.EXE at file offset 0x498f0, 0x70 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Picks a script of the drawn priority among those available ('No script p %d i:%d'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what GetScriptIndexByPriority does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
