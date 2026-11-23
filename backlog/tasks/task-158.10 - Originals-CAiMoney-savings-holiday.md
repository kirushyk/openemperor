---
id: TASK-158.10
title: Original's CAiMoney (savings holiday)
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 290000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CAiMoney (savings holiday), in GAME.EXE at file offset 0x399b0, 0x40 bytes; file offset 0x39a04, 0x2c bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Starts and stops a side's 'savings holiday' ('CAiMoney: side %d starting savings holiday'), in which it holds its money instead of spending it. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CAiMoney (savings holiday) does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
