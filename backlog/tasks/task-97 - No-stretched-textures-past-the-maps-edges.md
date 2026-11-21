---
id: TASK-97
title: No stretched textures past the map's edges
status: Done
assignee: []
created_date: '2026-11-21T17:20:00Z'
updated_date: '2026-11-21T17:27:00Z'
labels: []
dependencies: []
ordinal: 97000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
At the map's edges the ground's picture is stretched down from a single row of pixels, endlessly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Nothing stretched shows past the edges of the map
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Each map's model hangs walls straight down from its edges to -500, every corner at one point of their picture; the wash, looked up by where a point lies, stretched its edge row of pixels down them. Faces that lie along an edge of the map are now left out (1219 on the first Atreides map, 860 on the first skirmish map), and past the edge shows the background. Tested on made-up faces: walls along each edge go, ground in a corner stays.
<!-- SECTION:FINAL_SUMMARY:END -->
