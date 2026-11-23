---
id: TASK-141.2
title: Script function F7 GetExitPoint
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 179000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf05e9, 0xd0 bytes; +0x400000 in memory): GetExitPoint(side): position. Error 'ParserError ... Invalid line' if the side does not exist. In co-op, a human side is swapped for a random human partner (0x535220). Asks the map for the side's mission position (0x4ecd90, 'Mission Side (%d) position Request Failed') and converts it with 0x4ecca0 to the exit point.  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's GetExitPoint does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls GetExitPoint and checks the effect
<!-- AC:END -->
