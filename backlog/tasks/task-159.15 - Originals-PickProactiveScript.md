---
id: TASK-159.15
title: Original's PickProactiveScript
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 310000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ PickProactiveScript, in GAME.EXE at file offset 0x5b5c0, 0x1cb bytes; file offset 0x5ba22, 0x8e bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Chooses the side's next attack script: draws a priority, takes the scripts of it whose tech level, house and units allow, not one already running, and starts it as a tactic, building units on demand when it lacks them ('starting PROACTIVE script: %s with units built on demand'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what PickProactiveScript does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
