---
id: TASK-137
title: Units stand as close as the original's
status: Done
assignee: []
created_date: '2026-11-22T02:43:00Z'
updated_date: '2026-11-22T03:45:00Z'
labels: []
dependencies: []
ordinal: 137000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Units are spread too far apart. Infantry out of the barracks takes one cell; when another comes out, those already there step one cell aside.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Units sent or made keep one cell between them, as the original's
- [x] #2 A soldier out of the barracks stands on the cell by its door; those already there step one cell aside for the next
- [x] #3 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Spots are now as wide as what stands there (a square for a soldier, two for a machine, by the rules' Size) instead of 2.5 squares for all; units plan ways round buildings' solid squares and those standing still, and stop once held up within their own width of their spot. A soldier out of a building with no gathering point stands on the square by its door, those idle there stepping a square aside along its front.
<!-- SECTION:FINAL_SUMMARY:END -->
