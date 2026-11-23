---
id: TASK-171.3
title: Original's GetPlayerIndexOfAiPlayingHouse
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-171
ordinal: 341000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ GetPlayerIndexOfAiPlayingHouse, in GAME.EXE at file offset 0x91130, 0x28c bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Finds which player slot plays a house the AI runs in a campaign battle. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what GetPlayerIndexOfAiPlayingHouse does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
