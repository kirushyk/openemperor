---
id: TASK-141.8
title: Script function F25 ObjectNearToSideBase
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 185000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf1920, 0xe5 bytes; +0x400000 in memory): ObjectNearToSideBase(obj, side): true if the object is near the side's base: core 0x4ed9d0 (read below); co-op expansion on side. Helper 0x4ed9d0(obj, side): true if a building (square's occupant of kind 2, 0x54c650) on a square within 7 squares of the object (distance squared < 49) belongs to `side` and is seen by the object's owner (its seen-by mask +0x6c has the owner's bit). Used 31 times in 11 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's ObjectNearToSideBase does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls ObjectNearToSideBase and checks the effect
<!-- AC:END -->
