---
id: TASK-159.2
title: Original's AccessToSm
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 297000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ AccessToSm, in GAME.EXE at file offset 0x29040, 0x60 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Returns the strategy manager of a side's AI ('AccessToSm: Unknown Side %d'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what AccessToSm does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
