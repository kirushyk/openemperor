---
id: TASK-163.1
title: Original's COptionsStorage::Storing
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-163
ordinal: 337000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ COptionsStorage::Storing, in GAME.EXE at file offset 0x10f9d0, 0xac bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Writes the options to the registry under Software\Westwood\Emperor\Options: GraphicsLOD, ColorDepth, Shadows and the rest. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what COptionsStorage::Storing does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
