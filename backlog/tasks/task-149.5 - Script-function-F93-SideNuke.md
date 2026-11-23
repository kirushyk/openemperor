---
id: TASK-149.5
title: Script function F93 SideNuke
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:04:00Z'
labels: []
dependencies: []
parent_task_id: TASK-149
ordinal: 232000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf33f5, 0x74 bytes; +0x400000 in memory): SideNuke(side): unused; logs 'eSideNukeFunction: InvalidSide %d' and errors if the side does not exist; else Side::Nuke (0x53b4c0): marks the side (+0x18 = 1) and sets the health of every building and unit of it to 0, destroying them all (co-op: the whole human team).  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's SideNuke does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls SideNuke and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
SideNuke(side) and SideNukeAll() leave everything of the side, or of every side, with no health (Battle::nuke, through set_health 0). Tested in game.rs (a_side_nuked_has_nothing_left_and_nuking_all_leaves_nothing_of_anyone) and mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
