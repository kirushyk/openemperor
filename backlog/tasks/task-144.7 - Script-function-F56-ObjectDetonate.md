---
id: TASK-144.7
title: Script function F56 ObjectDetonate
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-144
ordinal: 204000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3248, 0x65 bytes; +0x400000 in memory): ObjectDetonate(obj, explosion): sets the object's health (+0x3c) to 0 and makes the explosion `explosion` (an [ExplosionTypes] kind) at its position, rotation and scale (object +0x14, +0x20, +0x2c) through the world (0x594c20).  Used 49 times in 11 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's ObjectDetonate does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls ObjectDetonate and checks the effect
<!-- AC:END -->
