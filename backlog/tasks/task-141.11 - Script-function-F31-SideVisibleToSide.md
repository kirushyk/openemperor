---
id: TASK-141.11
title: Script function F31 SideVisibleToSide
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 188000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf0b32, 0x1ff bytes; +0x400000 in memory): SideVisibleToSide(a, b): core 0x4edf00: true if any unit (Side::FirstUnit/NextUnit) or building (FirstBuilding/NextBuilding) of side a has side b's bit in its seen-by mask (+0x6c). Co-op expansion on either side.  Used 61 times in 32 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's SideVisibleToSide does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls SideVisibleToSide and checks the effect
<!-- AC:END -->
