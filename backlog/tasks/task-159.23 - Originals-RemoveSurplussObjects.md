---
id: TASK-159.23
title: Original's RemoveSurplussObjects
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 318000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ RemoveSurplussObjects, in GAME.EXE at file offset 0x5f290, 0x81 bytes; file offset 0x5f311, 0x11f bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Drops from a target the objects on its megatile that do not belong to the tactic's object set. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what RemoveSurplussObjects does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
