---
id: TASK-174.2
title: Original's FXManager::TriggerWeapon
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-174
ordinal: 332000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ FXManager::TriggerWeapon, in GAME.EXE at file offset 0xb3c00, 0x90 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: An effect event fires one of the object's weapons ('Trying to fire an invalid weapon'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what FXManager::TriggerWeapon does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
