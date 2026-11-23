---
id: TASK-141.17
title: Script function F145 GetIsolatedEntrance
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 267000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf40de, 0x54 bytes; +0x400000 in memory): GetIsolatedEntrance(pos): unused; calls the same 0x4ed380(pos) as GetNeutralExitPoint, so it gives the same point.  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's GetIsolatedEntrance does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls GetIsolatedEntrance and checks the effect
<!-- AC:END -->
