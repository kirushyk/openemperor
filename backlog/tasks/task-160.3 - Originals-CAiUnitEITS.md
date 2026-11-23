---
id: TASK-160.3
title: Original's CAiUnitEITS
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-160
ordinal: 325000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CAiUnitEITS, in GAME.EXE at file offset 0x66c7d, 0x103 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: The Ordos EITS's AI: finds a target to go to ('CAiUnitEITS: No target!'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CAiUnitEITS does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
