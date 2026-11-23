---
id: TASK-143.6
title: Script function F95 RadarEnabled
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-143
ordinal: 234000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf02bf, 0xe6 bytes; +0x400000 in memory): RadarEnabled(): no argument; sets byte +0x895 (radar on) of the player's side, or of every side of the human team in co-op. It never turns the radar off.  Used 3 times in 3 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's RadarEnabled does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls RadarEnabled and checks the effect
<!-- AC:END -->
