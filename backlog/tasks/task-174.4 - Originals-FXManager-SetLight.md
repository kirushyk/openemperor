---
id: TASK-174.4
title: Original's FXManager::SetLight
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-174
ordinal: 334000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ FXManager::SetLight, in GAME.EXE at file offset 0xb3d90, 0xd2 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: An effect event switches one of the object's lights (0x4b3920). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what FXManager::SetLight does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
