---
id: TASK-141.9
title: Script function F26 ObjectNearToObject
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:20:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 186000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf1a05, 0x107 bytes; +0x400000 in memory): ObjectNearToObject(a, b): true when the distance between the two objects' centres (virtual 0x204 on each, from their position +0x14) is less than (half of a's rules size + half of b's rules size + 5) x 32, the size being rules entry (+0x70) field +0x9c in squares.  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's ObjectNearToObject does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls ObjectNearToObject and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
ObjectNearToObject(a, b) is true when the two stand nearer than a's radius + b's radius + 5 squares (half a rules size taken as the collision radius, half the tiles across). Tested in game.rs (two_things_are_near_within_half_their_sizes_and_five_squares) and mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
