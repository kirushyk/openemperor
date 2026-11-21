---
id: TASK-107
title: Harvester eats as the original's
status: Done
assignee: []
created_date: '2026-11-21T19:56:00Z'
updated_date: '2026-11-21T20:28:00Z'
labels: []
dependencies: []
ordinal: 107000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Ours roams to random points at full speed and plays only Harv Eat Hold, so its scoop hardly moves; filling takes too long.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 At each spot it stops, lowers its scoop (Harv Eat Start), eats (Hold) and raises it (Harv Eat End) before moving on a short way
- [x] #2 It fills sooner than now
- [x] #3 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Collecting now: stopped on sand it eats for EAT_SECONDS (6), loading at 100/30 a second, then roams to the nearest unclaimed spot a short way ahead (else the nearest). The model plays Harv Eat Start, loops Hold and plays End across the 6 s; dust rises while it eats. Rates are guesses, logged.
<!-- SECTION:FINAL_SUMMARY:END -->
