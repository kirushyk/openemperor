---
id: TASK-144.17
title: Script function F158 SetTilePos
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-144
ordinal: 277000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf438a, 0x7d bytes; +0x400000 in memory): SetTilePos(x, y): returns the position of the centre of map square (x, y) (square 0x4e5e70, square to world 0x4e5e40).  Used 67 times in 5 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's SetTilePos does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls SetTilePos and checks the effect
<!-- AC:END -->
