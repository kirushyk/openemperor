---
id: TASK-159.1
title: Original's AbortLeastImportantScriptTactic
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 296000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ AbortLeastImportantScriptTactic, in GAME.EXE at file offset 0x5dc60, 0x120 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: When too many tactics run, aborts one: first a tactic still assembling, then one with no target, then one not in a conflict, then any ('Aborting tactic still assembling' ... 'Aborting any tactic'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what AbortLeastImportantScriptTactic does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
