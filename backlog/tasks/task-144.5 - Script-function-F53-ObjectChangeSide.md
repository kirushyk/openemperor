---
id: TASK-144.5
title: Script function F53 ObjectChangeSide
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-144
ordinal: 202000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf11c5, 0x33 bytes; +0x400000 in memory): ObjectChangeSide(obj, side): hands the object to the side through the world's capture routine (0x596d80(obj, side, 0), the one that logs 'CAPTURE %d %d').  Used 195 times in 33 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's ObjectChangeSide does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls ObjectChangeSide and checks the effect
<!-- AC:END -->
