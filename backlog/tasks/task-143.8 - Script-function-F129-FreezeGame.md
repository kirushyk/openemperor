---
id: TASK-143.8
title: Script function F129 FreezeGame
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:12:00Z'
labels: []
dependencies: []
parent_task_id: TASK-143
ordinal: 257000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3a5b, 0x12 bytes; +0x400000 in memory): FreezeGame(): sets the world's byte +0x1b4dc to 1 (World::SetFrozen 0x4f4e20); the world clears it when made and loaded (0x593a92, 0x593c51). By its name and use (a freeze, a scene, an unfreeze) the world stands still while the script runs on; the code that reads the flag was not found by its offset.  Used 69 times in 58 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's FreezeGame does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls FreezeGame and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
FreezeGame and UnFreezeGame set and clear Game::script_frozen; while it is set the battle's world (Game::advance_world: units, shots, bases, opponents) stands still and the script, clock and fog go on. Tested in game.rs (a_frozen_game_stands_still_while_its_script_runs_on) and mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
