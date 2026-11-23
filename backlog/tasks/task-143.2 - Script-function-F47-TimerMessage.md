---
id: TASK-143.2
title: Script function F47 TimerMessage
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-143
ordinal: 196000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf0200, 0x99 bytes; +0x400000 in memory): TimerMessage(seconds): formats the number as hours:minutes:seconds with '%2d:%02d:%02d' (seconds/3600, seconds/60 mod 60, seconds mod 60), creating the script's timer box on first use (0x4eb070, kept at script+0x390) and setting its text (0x4ea650). It does not count by itself: scripts call it each update with the time left.  Used 20 times in 9 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's TimerMessage does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls TimerMessage and checks the effect
<!-- AC:END -->
