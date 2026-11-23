---
id: TASK-141.5
title: Script function F15 GetSecondPlayerSide
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T17:59:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 182000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf0d95, 0xed bytes; +0x400000 in memory): GetSecondPlayerSide(): outside the co-op campaign logs 'Second Side Request (SinglePlayer)'. Then returns the first side of the human team that exists and is not the player's (co-op human) side; if none, the player's side (0x4ecaa0).  Used 23 times in 4 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's GetSecondPlayerSide does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls GetSecondPlayerSide and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Outside the co-op campaign, which we never are in, GetSecondPlayerSide is the player's side, as the original's fallback (0x4ecaa0) gives; its debug log is left out. Tested in mission.rs (a_game_is_never_the_co_op_campaign).
<!-- SECTION:FINAL_SUMMARY:END -->
