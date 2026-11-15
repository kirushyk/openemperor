---
id: TASK-11
title: Make the tornado look like the original's
status: Done
assignee: []
created_date: '2026-11-14T01:28:00Z'
updated_date: '2026-11-14T02:14:00Z'
labels: []
dependencies: []
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The tornado should look like the one in the original game.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The tornado is drawn with the disc's model and its picture slides, turning as the original's does
- [x] #2 It moves and does harm as before
- [x] #3 A test covers that it draws the model when the disc is mounted
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The disc's IN_Tornado model draws in place of our procedural funnel when the disc is mounted, and ours stays as the fallback without it. Pictures marked @ are now see-through veils, blended by their alpha even on ? marker parts, unless the part waits on a condition ({LEECH}). The model plays Surface as the tornado rises, loops Move, and plays Sink over its last six seconds; its pictures slide as the disc says. The simulation is unchanged, apart from new age/left accessors. Side effect: buildings' flat @Skirt2 ground skirts and the Atreides palace's @glass now show see-through. Tests cover veil gathering, decal drawing, the fallback, and the frame choice.
<!-- SECTION:FINAL_SUMMARY:END -->
