---
id: TASK-70.1
title: Original's CArtIni::AddItemXafConstruction
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-70
ordinal: 338000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CArtIni::AddItemXafConstruction, in GAME.EXE at file offset 0x6f7d0, 0x1a0 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Reads an art-list entry's XafConstruction: the model a building shows while it goes up. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CArtIni::AddItemXafConstruction does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
