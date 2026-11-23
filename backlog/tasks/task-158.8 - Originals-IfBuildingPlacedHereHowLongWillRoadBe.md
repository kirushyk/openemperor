---
id: TASK-158.8
title: Original's IfBuildingPlacedHereHowLongWillRoadBe
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 288000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ IfBuildingPlacedHereHowLongWillRoadBe, in GAME.EXE at file offset 0x372bd, 0x63 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Measures the road a building at a spot would need to reach the road net (0x4350b0), so placement prefers short roads ('max road length distance exceeded %d with %d'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what IfBuildingPlacedHereHowLongWillRoadBe does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
