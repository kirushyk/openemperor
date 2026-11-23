---
id: TASK-141.14
title: Script function F138 ObjectIsCarried
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 263000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3c46, 0x73 bytes; +0x400000 in memory): ObjectIsCarried(obj): unused by the disc's scripts; TRUE if the object's carrier handle (0x463500) is not -1, i.e. a carryall holds it.  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's ObjectIsCarried does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls ObjectIsCarried and checks the effect
<!-- AC:END -->
