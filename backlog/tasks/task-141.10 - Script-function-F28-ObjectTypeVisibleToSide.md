---
id: TASK-141.10
title: Script function F28 ObjectTypeVisibleToSide
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 187000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf09f6, 0xd5 bytes; +0x400000 in memory): ObjectTypeVisibleToSide(type, side): core 0x4edf80: by the kind's rules category (Rules::Entry(type)[0]: 2 building, 1 unit), true if any object of that kind belonging to any of the 16 sides other than `side` is in side's seen-by mask.  Used 3 times in 2 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's ObjectTypeVisibleToSide does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls ObjectTypeVisibleToSide and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
ObjectTypeVisibleToSide(kind, side) is true when a living thing of that kind (a yard an MCV dug in, as SideObjectCount has it) that is not the side's is in sight of any of the side's things (Game::in_sight_of, shared with SideVisibleToSide). Tested in game.rs (a_side_sees_a_kind_of_anothers_in_sight_of_its_own) and mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
