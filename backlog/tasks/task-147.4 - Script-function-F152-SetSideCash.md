---
id: TASK-147.4
title: Script function F152 SetSideCash
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-147
ordinal: 271000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf4212, 0x4c bytes; +0x400000 in memory): SetSideCash(side, n): sets the side's money to n (Side 0x53e3b0), but only while the world's tick (0x53d080) is 0, at the very start.  Used 1 times in 1 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's SetSideCash does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls SetSideCash and checks the effect
<!-- AC:END -->
