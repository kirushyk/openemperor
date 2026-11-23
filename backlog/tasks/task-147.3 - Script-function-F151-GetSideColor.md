---
id: TASK-147.3
title: Script function F151 GetSideColor
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-147
ordinal: 270000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf41ce, 0x44 bytes; +0x400000 in memory): GetSideColor(side): unused by the disc's scripts; the side's colour number (game 0x481520), 6 if the side does not exist.  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's GetSideColor does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls GetSideColor and checks the effect
<!-- AC:END -->
