---
id: TASK-159.10
title: Original's TacticAttack (CAiTacticAttack)
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 305000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ TacticAttack (CAiTacticAttack), in GAME.EXE at file offset 0x549a0, 0x10a bytes (+0x400000 in memory; the size runs to the next known function start). What it does: The attack tactic: sends its team at the main enemy side's objects; with none known, to a random place on the map ('TacticAttack: Sending to random position'); a mission attack with no enemy side cannot run. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what TacticAttack (CAiTacticAttack) does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
