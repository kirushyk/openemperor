---
id: TASK-159.12
title: Original's CalculateAdjustedFrequency
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 307000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CalculateAdjustedFrequency, in GAME.EXE at file offset 0x494ad, 0x43 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: A script's frequency, adjusted by how well it has done: average performance (50 if never run) x 0.1 - 5 is added, the result kept between half and twice its own frequency and at most 100 ('Side %d CalculateAdjustedFrequency: Script %s priority %d required freq was %d changed to %d'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CalculateAdjustedFrequency does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
