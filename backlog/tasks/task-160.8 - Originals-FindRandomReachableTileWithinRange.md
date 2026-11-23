---
id: TASK-160.8
title: Original's FindRandomReachableTileWithinRange
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-160
ordinal: 330000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ FindRandomReachableTileWithinRange, in GAME.EXE at file offset 0x6b743, 0x1fd bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Picks a random square within a range of a unit that it can reach. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what FindRandomReachableTileWithinRange does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
