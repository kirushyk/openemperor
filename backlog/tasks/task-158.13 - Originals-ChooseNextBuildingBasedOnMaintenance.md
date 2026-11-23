---
id: TASK-158.13
title: Original's ChooseNextBuildingBasedOnMaintenance
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 293000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ ChooseNextBuildingBasedOnMaintenance, in GAME.EXE at file offset 0x2fbd0, 0xe7 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Chooses the next building by the groups' ratios: for each group compares its actual ratio with the required one ('Group %d, actual ratio %f required ratio %f diff %f') and picks a building of the group that lags most, with a random choice inside the group, skipping sub-house kinds it cannot build. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what ChooseNextBuildingBasedOnMaintenance does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
