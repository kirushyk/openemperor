---
id: TASK-146.9
title: Script function F73 SideAIAttackObject
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:33:00Z'
labels: []
dependencies: []
parent_task_id: TASK-146
ordinal: 215000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf281c, 0x83 bytes; +0x400000 in memory): SideAIAttackObject(side, obj): MissionCommandAttackObject (0x4288c0 -> 0x44eca0): mode 8, an 'Attack ObjectT' tactic of all units against that object. Every SideAI order first checks the side exists, then (if the AI manager, game 0x808d14 field +0x5c, says the side is not AI-run, 0x4285b0) hands it to the AI (0x4283f0(side, 1)), then calls CAiSide's mission command. Each mission command (CAiStrategyManager 0x44ebd0-0x44eea2) clears the side's running tactics (0x45c000), sets its mission mode (+0x34) and starts one tactic for all its units; mode numbers: 6 aggressive, 7 move, 8 attack object, 9 guard object, 10 exit map, 11 enter building, 12 headless chicken, 13 shuffle. Used 18 times in 13 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's SideAIAttackObject does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls SideAIAttackObject and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
SideAIAttackObject stops the side (as SideAIStop) and sends each of its units that can fire on the thing after it (Unit::go_after). Tested in game.rs (a_side_told_to_attack_a_thing_goes_after_it) and mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
