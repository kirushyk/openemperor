---
id: TASK-143.9
title: Script function F130 UnFreezeGame
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:12:00Z'
labels: []
dependencies: []
parent_task_id: TASK-143
ordinal: 258000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3a6d, 0x12 bytes; +0x400000 in memory): UnFreezeGame(): sets the same byte back to 0.  Used 70 times in 59 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's UnFreezeGame does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls UnFreezeGame and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
FreezeGame and UnFreezeGame set and clear Game::script_frozen; while it is set the battle's world (Game::advance_world: units, shots, bases, opponents) stands still and the script, clock and fog go on. Tested in game.rs (a_frozen_game_stands_still_while_its_script_runs_on) and mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
