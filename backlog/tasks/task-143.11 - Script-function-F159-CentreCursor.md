---
id: TASK-143.11
title: Script function F159 CentreCursor
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-143
ordinal: 278000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3a98, 0xa bytes; +0x400000 in memory): CentreCursor(): unused; puts the mouse cursor in the middle of the game view (input 0x809830, 0x4d8940 with the view's width and height halved).  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's CentreCursor does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls CentreCursor and checks the effect
<!-- AC:END -->
