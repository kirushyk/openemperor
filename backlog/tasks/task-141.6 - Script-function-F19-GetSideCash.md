---
id: TASK-141.6
title: Script function F19 GetSideCash
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 183000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf1f6c, 0xa1 bytes; +0x400000 in memory): GetSideCash(side): the side's money, Side+0x868. In co-op, the sum over the human team's sides.  Used 2 times in 2 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's GetSideCash does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls GetSideCash and checks the effect
<!-- AC:END -->
