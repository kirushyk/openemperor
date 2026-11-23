---
id: TASK-149.6
title: Script function F94 SideNukeAll
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-149
ordinal: 233000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3469, 0x30 bytes; +0x400000 in memory): SideNukeAll(): Side::Nuke on every one of the 16 sides that exists.  Used 1 times in 1 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's SideNukeAll does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls SideNukeAll and checks the effect
<!-- AC:END -->
