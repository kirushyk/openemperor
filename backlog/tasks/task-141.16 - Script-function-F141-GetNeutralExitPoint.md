---
id: TASK-141.16
title: Script function F141 GetNeutralExitPoint
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 265000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf408a, 0x54 bytes; +0x400000 in memory): GetNeutralExitPoint(pos): returns a position from 0x4ed380(pos), which goes through the sides' mission positions (each side's table at +0x44, 0x53b530), skipping the player's (or co-op human) side, and logs 'Mission Side (%d) position Request Failed.' when none is found.  Used 1 times in 1 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's GetNeutralExitPoint does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls GetNeutralExitPoint and checks the effect
<!-- AC:END -->
