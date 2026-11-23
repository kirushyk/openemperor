---
id: TASK-149.7
title: Script function F98 ReplaceShroud
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:03:00Z'
labels: []
dependencies: []
parent_task_id: TASK-149
ordinal: 236000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf34d4, 0x28 bytes; +0x400000 in memory): ReplaceShroud(pos, n): for each human side, covers the map again with shroud n squares round pos (0x4ec820).  Used 7 times in 3 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's ReplaceShroud does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls ReplaceShroud and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
ReplaceShroud covers the player's fog again round a place (Fog::cover: opened, now and ever set back, to be seen anew by units there); RemoveMapShroud opens the whole map for good (Fog::uncover_all), as the original zeroes each human side's shroud. Tested in fog.rs and mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
