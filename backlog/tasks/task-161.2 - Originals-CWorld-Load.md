---
id: TASK-161.2
title: Original's CWorld::Load
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-161
ordinal: 336000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CWorld::Load, in GAME.EXE at file offset 0x197210, 0xd7 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Loads the world from the save's 'Model Block' (0x594060, 0x593bd0; 'TICONDATA IS %d'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CWorld::Load does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
