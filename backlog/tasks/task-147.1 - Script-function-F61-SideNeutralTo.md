---
id: TASK-147.1
title: Script function F61 SideNeutralTo
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:09:00Z'
labels: []
dependencies: []
parent_task_id: TASK-147
ordinal: 206000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf16ac, 0x18f bytes; +0x400000 in memory): SideNeutralTo(a, b): sets side a's relation to b to 2, neutral (Side 0x53b4a0(b, 2); friend and enemy are the other values F59/F60 set); co-op expansion on either side.  Used 65 times in 21 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's SideNeutralTo does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls SideNeutralTo and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
SideNeutralTo(a, b) ends the two sides' enmity, as our battle keeps only enemies and not (no separate neutral relation, and relations are both ways). Tested in mission.rs (a_prison_camps_sides_are_related_and_its_prisoners_freed_once_seen).
<!-- SECTION:FINAL_SUMMARY:END -->
