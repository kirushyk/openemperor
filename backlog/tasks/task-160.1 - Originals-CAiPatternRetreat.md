---
id: TASK-160.1
title: Original's CAiPatternRetreat
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-160
ordinal: 323000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CAiPatternRetreat, in GAME.EXE at file offset 0x3f410, 0x17c bytes (+0x400000 in memory; the size runs to the next known function start). What it does: When the side is losing badly it retreats ('AI Side %d Retreating') or, by chance, makes a last attack ('is losing so sending last gasp attack'): aborts all script and defence tactics and sends its units to a 'Retreat Point'. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CAiPatternRetreat does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
