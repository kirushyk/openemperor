---
id: TASK-13
title: Draw the water of the Caladan maps with our water shader
status: Done
assignee: []
created_date: '2026-11-14T01:28:00Z'
updated_date: '2026-11-14T05:00:00Z'
labels: []
dependencies: []
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Render water on the disc's maps with the water shader we had for our own maps.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The water on the disc's maps, such as the Caladan ones, is drawn with our water shader
- [x] #2 The land around it is drawn as before, and maps without water look as before
- [x] #3 A test covers that water cells on a disc map are drawn as water
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Faces of a disc map painted with sea.tga (Caladan) or icesea.tga (Draconis) now go into a new Frame.water list. They are flat sea surfaces with no bed (y −131 on Peninsula and Capital, −21..−4.5 on River Delta, −2.5 on Draconis). Each corner sits SEA_DEPTH (one tile diagonal) deep under it. Metal draws that list with our water shader (material 1) and runs the mirror pass only when there is water. The mirror is now built about the sea's height (Frame.sea, from the lowest sea face; View::mirror_matrix(height)). The mirror pass cuts at that height (Sky.sea) instead of 0. All other ground is drawn as before as material 0, and maps without sea have no water list and no mirror pass. On the disc: Koth Crater has 0 water vertices, Peninsula 5454, River Delta 2655, Draconis Plain 1047. The shader compiles offline. Tests: lay() sends sea faces to the water, deep, and land to the ground; the mirror matrix about a lower sea.
<!-- SECTION:FINAL_SUMMARY:END -->
