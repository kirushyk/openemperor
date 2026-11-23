---
id: TASK-141.18
title: Script function F148 GetValley
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 268000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf4132, 0x6a bytes; +0x400000 in memory): GetValley(a, b): unused; over the map's zones of type 1 (valleys; map data 0x4b6510 count, 0x4b6530 zone), takes each zone's point (0x4ec250) that passes the test 0x4ec370 against a and b and keeps the one nearest; returns it (a when none).  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's GetValley does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls GetValley and checks the effect
<!-- AC:END -->
