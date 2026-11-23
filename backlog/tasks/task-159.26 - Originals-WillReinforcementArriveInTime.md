---
id: TASK-159.26
title: Original's WillReinforcementArriveInTime
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 321000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ WillReinforcementArriveInTime, in GAME.EXE at file offset 0x59b20, 0xb0 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Whether a unit can still join a forming team: yes if it is within 32 squares of the team's point (distance squared under 1024). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what WillReinforcementArriveInTime does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
