---
id: TASK-149.1
title: Script function F80 SideAttractsWorms
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-149
ordinal: 222000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf2a99, 0xa9 bytes; +0x400000 in memory): SideAttractsWorms(side): sets the side's byte +0x894 to 1 (0x53f700), which the worm manager reads to send worms after its units; co-op expansion.  Used 1 times in 1 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's SideAttractsWorms does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls SideAttractsWorms and checks the effect
<!-- AC:END -->
