---
id: TASK-141.4
title: Script function F10 GetEntranceNearToPos
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 181000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf0776, 0x55 bytes; +0x400000 in memory): GetEntranceNearToPos(pos): the map entrance nearest to pos: starts with entrance 0 and keeps whichever entrance has the smaller distance (0x575a30) to pos (helper 0x4ecb70).  Used 429 times in 24 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's GetEntranceNearToPos does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls GetEntranceNearToPos and checks the effect
<!-- AC:END -->
