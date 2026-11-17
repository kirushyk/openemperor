---
id: TASK-35
title: First Atreides mission on its map after its movie
status: Done
assignee: []
created_date: '2026-11-17T07:51:00Z'
updated_date: '2026-11-17T07:51:00Z'
labels: []
dependencies: []
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Confirming a house plays its opening movie from its disc, then its first battle on the original's start map.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Atreides open with A00_F00E from disc 2, then #u1
- [x] #2 Without the disc it goes straight on
- [x] #3 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Screen::Opening(house) after ConfirmFaction; main plays the house's Phase0a movie (A00_F00E / O00_F00E / H01_F00E) found by discs::movie inside the house disc's MOVIES0001.RFD as an FFmpeg subfile, verified to open on disc 2. Skipping or its end starts the campaign on maps::campaign_start (#u1/#u2/#u3, now always read) with 10 000 credits. The mission script itself is not run (logged).
<!-- SECTION:FINAL_SUMMARY:END -->
