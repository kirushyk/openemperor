---
id: TASK-161.1
title: Original's CWorld::Save
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-161
ordinal: 335000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CWorld::Save, in GAME.EXE at file offset 0x196ff0, 0x9c bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Saves the world to the save file's model block (0x53f050). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CWorld::Save does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
