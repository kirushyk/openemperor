---
id: TASK-149.2
title: Script function F81 SideRepelsWorms
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-149
ordinal: 223000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf2b42, 0xc0 bytes; +0x400000 in memory): SideRepelsWorms(side): unused; the same byte set to 0.  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's SideRepelsWorms does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls SideRepelsWorms and checks the effect
<!-- AC:END -->
