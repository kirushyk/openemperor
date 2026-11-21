---
id: TASK-113
title: Map's edges fade to black
status: Done
assignee: []
created_date: '2026-11-21T20:40:00Z'
updated_date: '2026-11-21T20:43:00Z'
labels: []
dependencies: []
ordinal: 113000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The land stops hard at the map's edge; the original shades it down to black towards the edge.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Near each edge of the map the land darkens smoothly to black at the edge
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
battlefield::edge_light fades a corner's light by smoothstep over the last 4 squares to each edge; the disc's land, its spice and our plain land take it as their seen. Width is a guess; units are not darkened.
<!-- SECTION:FINAL_SUMMARY:END -->
