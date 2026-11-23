---
id: TASK-159.7
title: Original's CAiSideShared
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 302000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CAiSideShared, in GAME.EXE at file offset 0x4c900, 0x130 bytes; file offset 0x4ca30, 0x127 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: What all AI sides share: the location manager and map data, saved under AiSidSh. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CAiSideShared does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
