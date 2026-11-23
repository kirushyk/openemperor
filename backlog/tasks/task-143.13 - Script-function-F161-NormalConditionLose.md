---
id: TASK-143.13
title: Script function F161 NormalConditionLose
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-143
ordinal: 280000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf441b, 0x77 bytes; +0x400000 in memory): NormalConditionLose(side): a query, not a setting: TRUE if the side has lost by the game's usual defeat test (game 0x808d14 side state 0x434320, then 0x57aa80, the win/lose detector with 'Do not know how to detect win condition'); in co-op for the human side, 0x4edb30 tests the whole human team.  Used 12 times in 10 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's NormalConditionLose does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls NormalConditionLose and checks the effect
<!-- AC:END -->
