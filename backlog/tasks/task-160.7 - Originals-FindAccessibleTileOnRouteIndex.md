---
id: TASK-160.7
title: Original's FindAccessibleTileOnRouteIndex
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-160
ordinal: 329000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ FindAccessibleTileOnRouteIndex, in GAME.EXE at file offset 0x6b9d7, 0x293 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Finds a reachable square along a route for a unit. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what FindAccessibleTileOnRouteIndex does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
