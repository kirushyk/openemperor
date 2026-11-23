---
id: TASK-158.14
title: Original's SetNoRoomAvailableForBuilding
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 294000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ SetNoRoomAvailableForBuilding, in GAME.EXE at file offset 0x2f777, 0x59 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Marks that a building kind cannot be placed for want of room ('Side %d No room available for %s'), so the queue gives it up ('Giving up trying to place building!!!'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what SetNoRoomAvailableForBuilding does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
