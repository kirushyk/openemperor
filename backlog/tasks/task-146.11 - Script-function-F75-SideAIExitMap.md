---
id: TASK-146.11
title: Script function F75 SideAIExitMap
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:37:00Z'
labels: []
dependencies: []
parent_task_id: TASK-146
ordinal: 217000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf29b4, 0x62 bytes; +0x400000 in memory): SideAIExitMap(side): MissionCommandExitByNearestEntrance (0x428850 -> 0x44ed80): mode 10, a 'Move Off Map T' tactic: every unit drives to its nearest map entrance and leaves the map (AiTacticMoveOffMap.cpp; 'Team has exited'). Every SideAI order first checks the side exists, then (if the AI manager, game 0x808d14 field +0x5c, says the side is not AI-run, 0x4285b0) hands it to the AI (0x4283f0(side, 1)), then calls CAiSide's mission command. Each mission command (CAiStrategyManager 0x44ebd0-0x44eea2) clears the side's running tactics (0x45c000), sets its mission mode (+0x34) and starts one tactic for all its units; mode numbers: 6 aggressive, 7 move, 8 attack object, 9 guard object, 10 exit map, 11 enter building, 12 headless chicken, 13 shuffle. Used 141 times in 66 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's SideAIExitMap does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls SideAIExitMap and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
SideAIExitMap stops the side and sends each of its units to the map entrance nearest it (units::nearest_exit); each update a leaving unit within 3 squares of an entrance goes off the map (units::at_an_exit, Game::leave_the_map, part of the world's update), not counted as lost; once all have gone SideAIDone holds, as the original's 'Team has exited'. Tested in units.rs (a_unit_leaves_by_the_nearest_entrance_once_it_comes_to_it), game.rs and mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
