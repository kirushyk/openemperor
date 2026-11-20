---
id: TASK-86
title: Mission speech numbers counted as the original counts them
status: Done
assignee: []
created_date: '2026-11-20T20:17:00Z'
updated_date: '2026-11-20T20:17:00Z'
labels: []
dependencies: []
ordinal: 86000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Scripts name a line of speech by its place among the entries of E_Output_Pickup.txt's MissionMessages sections, from 502; counting every line from 429 only held for the Atreides' first mission, and sent the tutorial three lines off and the Harkonnen and Ordos first missions to other lines altogether.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Numbers count only the entries of MissionMessages sections, from 502
- [x] #2 The Atreides, Harkonnen and Ordos first missions and the tutorial open with their own welcomes, checked on the disc
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
sounds.rs counts the entries of MissionMessages sections from 502; the disc test checks ATstart1, ATtutorial1, HKstart1 and ORstart1. Numbers below 502 (some territory scripts use them) still say nothing.
<!-- SECTION:FINAL_SUMMARY:END -->
