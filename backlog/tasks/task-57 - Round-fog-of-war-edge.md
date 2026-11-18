---
id: TASK-57
title: Round fog of war edge
status: Done
assignee: []
created_date: '2026-11-18T15:09:00Z'
updated_date: '2026-11-18T15:17:00Z'
labels: []
dependencies: []
ordinal: 57000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The fog's edge follows the ground's triangles; the original's is circular around what sees.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The edge of what is seen is round around each unit's sight, not stepped by ground triangles
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fog keeps signed distances (now, ever) per grid point; ground vertices carry them (WorldVertex.fog) and the Metal shader smoothsteps them per pixel. Frame checked.
<!-- SECTION:FINAL_SUMMARY:END -->
