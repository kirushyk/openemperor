---
id: TASK-159.6
title: Original's CAiSide
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 301000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CAiSide, in GAME.EXE at file offset 0x4acf0, 0x40 bytes; file offset 0x4ad30, 0x40 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: A computer side: its strategy manager, saved and loaded with it ('CAiSide: Saving side %d'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CAiSide does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
