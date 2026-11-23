---
id: TASK-141.7
title: Script function F20 GetSideSpice
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 184000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf200d, 0xbe bytes; +0x400000 in memory): GetSideSpice(side): the same field as GetSideCash, Side+0x868 (spice and credits are one figure); summed over the human team in co-op.  Used 4 times in 2 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's GetSideSpice does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls GetSideSpice and checks the effect
<!-- AC:END -->
