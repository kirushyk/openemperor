---
id: TASK-159.16
title: Original's LoadScriptFile
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 311000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ LoadScriptFile, in GAME.EXE at file offset 0x45870, 0x4c bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Reads AI_scripts.txt: registers the parser's keywords and reads the scripts ('Loaded %d scripts'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what LoadScriptFile does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
