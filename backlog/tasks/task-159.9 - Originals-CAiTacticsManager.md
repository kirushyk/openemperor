---
id: TASK-159.9
title: Original's CAiTacticsManager
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 304000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CAiTacticsManager, in GAME.EXE at file offset 0x5c0d0, 0x116 bytes; file offset 0x5c1e6, 0x9a bytes; file offset 0x5c280, 0x1fd bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Runs a side's tactics and its units' AI (AiU%02d%d), and its script picker (AiSPick%d); saved under AiTacMan%d. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CAiTacticsManager does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
