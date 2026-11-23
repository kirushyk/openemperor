---
id: TASK-159.19
title: Original's GetAnAssemblyPointLocation
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 314000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ GetAnAssemblyPointLocation, in GAME.EXE at file offset 0x2b980, 0xa0 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Returns an assembly point by kind: tactic, defensive or safe point, by index. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what GetAnAssemblyPointLocation does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
