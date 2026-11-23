---
id: TASK-141.13
title: Script function F34 SideNearToPoint
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:05:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 190000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf1ea6, 0xc6 bytes; +0x400000 in memory): SideNearToPoint(side, pos): true if any unit or building of the side stands within 5 squares of pos (distance squared, 0x575ab0, under 25600 = 160^2); in co-op for the human side, 0x4ede40 tests every human partner.  Used 91 times in 54 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's SideNearToPoint does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls SideNearToPoint and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
SideNearToPoint(side, pos) is true when any living unit or building of the side stands less than 5 squares (160 world units) from pos (Battle::side_near). Tested in mission.rs (a_side_is_near_a_place_within_five_squares).
<!-- SECTION:FINAL_SUMMARY:END -->
