---
id: TASK-144.16
title: Script function F137 NewObjectInAPC
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-144
ordinal: 262000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3bb1, 0x95 bytes; +0x400000 in memory): NewObjectInAPC(side, kind, apc): creates a unit of `kind` for `side` inside the APC object (World 0x5954e0; co-op human side: through the APC, 0x443d90) and returns the new unit as an obj (0 if the APC is gone).  Used 60 times in 5 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's NewObjectInAPC does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls NewObjectInAPC and checks the effect
<!-- AC:END -->
