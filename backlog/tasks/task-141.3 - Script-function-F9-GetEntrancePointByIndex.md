---
id: TASK-141.3
title: Script function F9 GetEntrancePointByIndex
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T17:52:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 180000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf06e8, 0x8e bytes; +0x400000 in memory): GetEntrancePointByIndex(i): position of the map's entrance i (map data 0x44a520 -> entrance list 0x4b5980); if i is out of range (count 0x45ff70) it returns entrance 0.  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's GetEntrancePointByIndex does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls GetEntrancePointByIndex and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Map entrances are read from each map's Entrance area (battlefield::Terrain::entrances) and the script picks them as the original's 0x4ecb70/0x4ecca0 do; tested in mission.rs (a_script_asks_for_the_maps_entrances_by_side_number_and_how_far_from_a_place).
<!-- SECTION:FINAL_SUMMARY:END -->
