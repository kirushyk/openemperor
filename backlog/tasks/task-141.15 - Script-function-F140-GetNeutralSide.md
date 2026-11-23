---
id: TASK-141.15
title: Script function F140 GetNeutralSide
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 264000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3e84, 0x16 bytes; +0x400000 in memory): GetNeutralSide(): returns side 15, the last of the 16 (0x468710 returns the constant 0xf).  Used 2 times in 2 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's GetNeutralSide does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls GetNeutralSide and checks the effect
<!-- AC:END -->
