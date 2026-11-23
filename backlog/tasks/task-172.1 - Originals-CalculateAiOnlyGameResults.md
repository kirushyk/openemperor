---
id: TASK-172.1
title: Original's CalculateAiOnlyGameResults
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-172
ordinal: 342000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CalculateAiOnlyGameResults, in GAME.EXE at file offset 0x93850, 0x19a bytes; file offset 0x939ea, 0x1f6 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Works out a battle between two computer houses without fighting it: scores for attacker and defender from their forces and a random factor ('Attacker Score %d Defender Score %d'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CalculateAiOnlyGameResults does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
