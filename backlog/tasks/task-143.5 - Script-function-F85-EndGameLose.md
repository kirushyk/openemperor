---
id: TASK-143.5
title: Script function F85 EndGameLose
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-143
ordinal: 225000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf31c9, 0x16 bytes; +0x400000 in memory): EndGameLose(): ends the game as a defeat for the player's side (game 0x808d14 method 0x481060 with the player or co-op human side).  Used 32 times in 17 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's EndGameLose does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls EndGameLose and checks the effect
<!-- AC:END -->
