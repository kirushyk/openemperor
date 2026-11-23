---
id: TASK-158.11
title: Original's ReallocateMoney
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 291000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ ReallocateMoney, in GAME.EXE at file offset 0x2f060, 0x100 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Sets how the AI splits its money by its construction mode (+0x48, 0-5): each mode passes a pair of figures to the money manager (0x439580): 'all money to units' (0, 100) and (100, 0), 'majority money to buildings' (20, 80), 'money evenly' (50, 60), 'majority money to units' (70, 30), 'most money to units' (70, 50). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what ReallocateMoney does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
