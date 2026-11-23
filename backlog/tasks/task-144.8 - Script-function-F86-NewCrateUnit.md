---
id: TASK-144.8
title: Script function F86 NewCrateUnit
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-144
ordinal: 226000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf332f, 0x58 bytes; +0x400000 in memory): NewCrateUnit(pos, kind): makes an object of the kind named "crate" (0x5974c0) at pos (World 0x5953e0), sets its crate type to 2 (unit, 0x488460) and the unit kind it holds to `kind` (0x4f4e30), and its health to -1.  Used 12 times in 3 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's NewCrateUnit does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls NewCrateUnit and checks the effect
<!-- AC:END -->
