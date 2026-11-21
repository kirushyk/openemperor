---
id: TASK-105
title: Scripted buildings kept inside the map
status: Done
assignee: []
created_date: '2026-11-21T19:56:00Z'
updated_date: '2026-11-21T19:58:00Z'
labels: []
dependencies: []
ordinal: 105000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The tutorial's refinery runs from column 55 to 65 on a 64-wide map; the rock it should stand on ends at 63.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A building a script or map puts down that would reach past the map's edge is moved in to stand within it
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
put() keeps a building's (not a prop's) squares within the map's cells, so the tutorial's refinery stands on columns 53-63, the rock right of its yard, rather than 55-65.
<!-- SECTION:FINAL_SUMMARY:END -->
