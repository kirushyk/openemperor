---
id: TASK-159.17
title: Original's ProcessScriptCommand
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 312000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ ProcessScriptCommand, in GAME.EXE at file offset 0x45fd0, 0xbf bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Parses one AI-script command: OBJECTSET, DESCRIPTION, TEAM, TARGET or STAGING ('ProcessScriptCommand: Unknown token %d'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what ProcessScriptCommand does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
