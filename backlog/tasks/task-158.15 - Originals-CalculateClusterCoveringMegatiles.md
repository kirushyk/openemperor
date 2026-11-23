---
id: TASK-158.15
title: Original's CalculateClusterCoveringMegatiles
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 295000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CalculateClusterCoveringMegatiles, in GAME.EXE at file offset 0x2bda8, 0x158 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: For each base cluster, works out the megatiles (map blocks) it covers, small and large ('%d small %d large'), which the defence and assembly points use. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CalculateClusterCoveringMegatiles does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
