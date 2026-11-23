---
id: TASK-146.8
title: Script function F72 SideAIStop
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:31:00Z'
labels: []
dependencies: []
parent_task_id: TASK-146
ordinal: 214000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf274f, 0xcd bytes; +0x400000 in memory): SideAIStop(side): MissionCommandAbort (0x428c00): the side's mission tactics are dropped and its units stop. Every SideAI order first checks the side exists, then (if the AI manager, game 0x808d14 field +0x5c, says the side is not AI-run, 0x4285b0) hands it to the AI (0x4283f0(side, 1)), then calls CAiSide's mission command. Each mission command (CAiStrategyManager 0x44ebd0-0x44eea2) clears the side's running tactics (0x45c000), sets its mission mode (+0x34) and starts one tactic for all its units; mode numbers: 6 aggressive, 7 move, 8 attack object, 9 guard object, 10 exit map, 11 enter building, 12 headless chicken, 13 shuffle. Used 41 times in 22 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's SideAIStop does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls SideAIStop and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
SideAIStop drops the side's hunt (script_hunting) and stops each of its units where it stands (Unit::stop, split out of order_stop, the S key's). Tested in game.rs (a_side_told_to_stop_gives_up_where_it_was_sent_and_its_hunt) and mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
