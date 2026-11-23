---
id: TASK-145.1
title: Script function F156 SetVeterancy
status: Done
assignee: []
created_date: '2026-11-23T15:23:00Z'
updated_date: '2026-11-23T18:07:00Z'
labels: []
dependencies: []
parent_task_id: TASK-145
ordinal: 275000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf4301, 0x42 bytes; +0x400000 in memory): SetVeterancy(obj, n): raises the object's rank n times by the routine that adds one rank (0x4c35a0, which warns 'Trying to increment to a non-existent veteran level'), so n is added to its rank, not set; nothing for n <= 0.  Used 1319 times in 51 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's SetVeterancy does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls SetVeterancy and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
SetVeterancy(obj, n) raises the unit n levels (Unit::raise_rank, as the original's 0x4c35a0: score set to the next level's, health share kept, nothing past the last level); nothing for n <= 0. Tested in units.rs and mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
