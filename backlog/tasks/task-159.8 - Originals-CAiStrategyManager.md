---
id: TASK-159.8
title: Original's CAiStrategyManager
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 303000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CAiStrategyManager, in GAME.EXE at file offset 0x4f6d0, 0xbd bytes; file offset 0x4f78d, 0xe3 bytes; file offset 0x4f870, 0x1a0 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: A side's strategy: its tactics manager, construction manager, pattern manager (AiPMan) and target detector (AiTarDet), saved under AiSMan%d; also the mission commands (see TASK-146). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CAiStrategyManager does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
