---
id: TASK-160.6
title: Original's FindAccessibleNear and FindAccessibleTileNearTileIndex
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-160
ordinal: 328000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ FindAccessibleNear and FindAccessibleTileNearTileIndex, in GAME.EXE at file offset 0x6befe, 0x72 bytes; file offset 0x6bcd0, 0x22e bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Finds a square near another that the unit's kind can reach (warning if none). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what FindAccessibleNear and FindAccessibleTileNearTileIndex does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
