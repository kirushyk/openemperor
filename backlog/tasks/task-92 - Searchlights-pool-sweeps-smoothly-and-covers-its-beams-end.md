---
id: TASK-92
title: Searchlight's pool sweeps smoothly and covers its beam's end
status: Done
assignee: []
created_date: '2026-11-21T05:45:00Z'
updated_date: '2026-11-21T05:46:00Z'
labels: []
dependencies: []
ordinal: 92000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The pool of light a searchlight throws jumps as it sweeps and is much smaller than the end of its beam.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The pool moves evenly between the model's frames, with no leaps
- [x] #2 The pool is as wide as the far end of the beam
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The pool now sits at the middle of the beam's far end, a hexagon on the Harkonnen compound's searchlights, instead of its farthest corner, which leapt from corner to corner as the beam turned; it is blended between the model's frames and reaches 1.5 times as far as that end spreads (at least the old reach, for vehicles' lamps). Tested on the disc: steps of under 2 units each 0.01 s, and an end over 100 wide.
<!-- SECTION:FINAL_SUMMARY:END -->
