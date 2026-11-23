---
id: TASK-149.9
title: Script function F153 AirStrike
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-149
ordinal: 272000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf425e, 0x5d bytes; +0x400000 in memory): AirStrike(side, pos, target, kind, kind, ...): in co-op a human side becomes a random partner. Core 0x4ee150: makes aircraft of the listed kinds for `side` arriving at pos (0x594390), logs 'eAirStrike: Side was %d', uncovers the shroud, makes the strike side and the target's side enemies both ways (0x53b4a0(.., 1)), AI-controls the strike side and orders MissionCommandAttackObject on the target.  Used 8 times in 7 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's AirStrike does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls AirStrike and checks the effect
<!-- AC:END -->
