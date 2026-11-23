---
id: TASK-139
title: Map the original's GAME.EXE against what we have
status: Done
assignee: []
created_date: '2026-11-23T14:28:00Z'
updated_date: '2026-11-23T14:36:00Z'
labels: []
dependencies: []
ordinal: 139000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Read GAME.EXE (from INSTALL/Game1.cab, 2 449 408 bytes, linked 2001-05-11) for its tables, file names and messages, and set down in docs/game-exe.md what the original has, where it is in the file (offset and size) and whether we have it. Then put each missing piece in the backlog with those references.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/game-exe.md maps GAME.EXE's sections, tables and subsystems with file offsets and sizes
- [x] #2 Each subsystem the map shows as missing or partial has a backlog task citing its offsets
- [x] #3 tok-format.md names the script functions and keywords from the executable's own table
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Extracted GAME.EXE from Game1.cab and mapped it from its strings, the code that refers to them, call and jump-table targets, and its switches. docs/game-exe.md has the sections (file offset = address - 0x400000), the tables (script names 0x1fe0b0, interpreter switch 0xf0011/0xf4a00, Rules keywords 0x20e79c, ai.ini, AI scripts, animations, cursors, events, keys, options, sounds, camera, territories), the data files it reads and 33 subsystems with anchor and code offsets, each against what we have. tok-format.md now names all 181 functions and keywords from the executable, with each one's case offset and size and its use count over the 228 mission scripts; three functions are no-ops in the original. Every offset cited was checked by script against the file. Missing pieces are TASK-140 to TASK-177; GAME.EXE references were added to TASK-16, 27, 28, 34, 70, 87, 129, 134 and 135. No code changed, so no tests were run.
<!-- SECTION:FINAL_SUMMARY:END -->
