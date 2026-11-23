---
id: TASK-149.3
title: Script function F82 ForceWormStrike
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-149
ordinal: 224000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf2c02, 0x1f bytes; +0x400000 in memory): ForceWormStrike(pos): the game's worm manager (0x44a800) makes a worm strike at pos now (0x4e3220 -> 0x46dbc0).  Used 8 times in 5 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's ForceWormStrike does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls ForceWormStrike and checks the effect
<!-- AC:END -->
