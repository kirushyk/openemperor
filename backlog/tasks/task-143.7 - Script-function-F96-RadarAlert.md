---
id: TASK-143.7
title: Script function F96 RadarAlert
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-143
ordinal: 235000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf03a5, 0x27 bytes; +0x400000 in memory): RadarAlert(pos): flashes a radar alert at pos (0x519b70(pos, 0, 0)).  Used 619 times in 149 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's RadarAlert does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls RadarAlert and checks the effect
<!-- AC:END -->
