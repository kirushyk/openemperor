---
id: TASK-149.4
title: Script function F92 SetReinforcements
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-149
ordinal: 231000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf2ea7, 0x21 bytes; +0x400000 in memory): SetReinforcements(side, n): stores n in the side's entry of the reinforcements table (game 0x4c3200, entry side x 20 + 0x10, 0x523580): the value CReinforcements reads to bring that side's reinforcements or not.  Used 21 times in 19 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's SetReinforcements does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls SetReinforcements and checks the effect
<!-- AC:END -->
