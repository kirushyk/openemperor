---
id: TASK-158.4
title: Original's AiIni (personality)
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 284000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ AiIni (personality), in GAME.EXE at file offset 0x31f70, 0x207 bytes; file offset 0x32177, 0x29f bytes; file offset 0x32510, 0x60 bytes; file offset 0x32570, 0x60 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Sets an AI side's personality ('** Setting AI side %d personality to %d **'): STRONG, WEAK, AGGRESSIVE or DEFENSIVE, each raising some of its ai.ini figures by a percentage ('AiIni: D Type %d, increasing by %d percent to give %d from %d', and S types likewise). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what AiIni (personality) does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
