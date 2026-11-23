---
id: TASK-174.1
title: Original's FXManager::TriggerSound
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-174
ordinal: 331000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ FXManager::TriggerSound, in GAME.EXE at file offset 0xb3b10, 0xcd bytes; file offset 0xb3bdd, 0x23 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Plays the sound an effect event names, at its object ('Playing sound associated with %s'; 'Null sound name.'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what FXManager::TriggerSound does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
