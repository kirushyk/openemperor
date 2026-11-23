---
id: TASK-171.4
title: Original's CMissionScreenManager
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-171
ordinal: 344000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CMissionScreenManager, in GAME.EXE at file offset 0xf7620, 0x110 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: A screen listing the mission files (AT*.txt, OR*.txt, HK*.txt) for picking the mission to play ('Set mission to %s', 'MissionUseThis', 'MissionIgnore'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CMissionScreenManager does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
