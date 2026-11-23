---
id: TASK-141.12
title: Script function F33 SideNearToSideBase
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 189000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf1ca1, 0x205 bytes; +0x400000 in memory): SideNearToSideBase(a, b): true if any unit or building of side a is within 7 squares of a building of side b that a's side can see (0x4ed9d0 for each of a's objects); co-op expansion on either side. Helper 0x4ed9d0(obj, side): true if a building (square's occupant of kind 2, 0x54c650) on a square within 7 squares of the object (distance squared < 49) belongs to `side` and is seen by the object's owner (its seen-by mask +0x6c has the owner's bit). Used 40 times in 17 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's SideNearToSideBase does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls SideNearToSideBase and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
ObjectNearToSideBase and SideNearToSideBase ask Battle::near_base: a thing (or any of a side's things) within 7 squares of the edge of a building of the other side that its own side has in sight (Game::in_sight_of standing in for the seen-by mask). Tested in game.rs (a_thing_near_a_building_of_a_side_it_sees_is_near_its_base) and mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
