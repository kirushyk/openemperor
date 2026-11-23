---
id: TASK-147.2
title: Script function F150 SetSideColor
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-147
ordinal: 269000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf419c, 0x32 bytes; +0x400000 in memory): SetSideColor(side, colour): if the side exists, the game sets its colour to the skirmish colour number (game 0x481480).  Used 202 times in 60 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's SetSideColor does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls SetSideColor and checks the effect
<!-- AC:END -->
