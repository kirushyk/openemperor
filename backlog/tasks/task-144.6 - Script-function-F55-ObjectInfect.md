---
id: TASK-144.6
title: Script function F55 ObjectInfect
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-144
ordinal: 203000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf320b, 0x3d bytes; +0x400000 in memory): ObjectInfect(obj, a, b): calls the object's virtual +0x54 (a, b, 100, its current health). For units (0x563a10): unless already infected (virtual +0x50) it stores the four values at +0x1f0..+0x1fc and puts the unit into state 0x40 (infected), retrying up to 5 times; the local player's unit gives a warning line (dialog 20 or 21 by its rules flag +0x383).  Used 272 times in 15 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's ObjectInfect does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls ObjectInfect and checks the effect
<!-- AC:END -->
