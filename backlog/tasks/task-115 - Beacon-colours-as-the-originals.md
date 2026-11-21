---
id: TASK-115
title: Beacon colours as the original's
status: Done
assignee: []
created_date: '2026-11-21T20:57:00Z'
updated_date: '2026-11-21T21:22:00Z'
labels: []
dependencies: []
ordinal: 115000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The beacon's smoke colour is our own guess.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The beacon looks as the original's does, from what the disc gives
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The beacon model's FXDataHeader names effect 269 on #smoke: start colour (255,255,255) and per-frame change (-17,-7,-17, alpha -17), as the green, blue and lava smokes' (128,255,128), (128,130,255) and white confirm the fields. Wisps gained a colour shift per second; beacon puffs now start white and turn green over 0.6 s, 25 a second. Read by hand, not parsed from the model.
<!-- SECTION:FINAL_SUMMARY:END -->
