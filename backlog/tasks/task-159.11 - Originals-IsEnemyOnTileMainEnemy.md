---
id: TASK-159.11
title: Original's IsEnemyOnTileMainEnemy
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 306000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ IsEnemyOnTileMainEnemy, in GAME.EXE at file offset 0x54579, 0x257 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Whether the objects on a target square belong to the tactic's main enemy side. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what IsEnemyOnTileMainEnemy does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
