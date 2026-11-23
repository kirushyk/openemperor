---
id: TASK-149.10
title: Script function F154 AirStrikeDone
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-149
ordinal: 273000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf42bb, 0x25 bytes; +0x400000 in memory): AirStrikeDone(side): reads the side's AI mission mode (0x428650): mode 8 (still attacking) -> orders MissionCommandExitByNearestEntrance and returns FALSE; mode 10 (leaving) -> TRUE; otherwise FALSE.  Used 5 times in 5 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's AirStrikeDone does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls AirStrikeDone and checks the effect
<!-- AC:END -->
