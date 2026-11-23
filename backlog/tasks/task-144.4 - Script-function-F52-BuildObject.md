---
id: TASK-144.4
title: Script function F52 BuildObject
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-144
ordinal: 201000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf10ed, 0x65 bytes; +0x400000 in memory): BuildObject(side, kind): the kind must be a unit (rules category 1), else 'ParserError ... Invalid line'. Issues a build order for the side through the order queue (0x4c44f0, the same path as a player's order), so its factory makes the unit as if ordered.  Used 60 times in 20 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's BuildObject does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls BuildObject and checks the effect
<!-- AC:END -->
