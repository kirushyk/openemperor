---
id: TASK-1
title: Harvesters that stop by the sand go back to work
status: Done
assignee: []
created_date: '2026-11-14T01:28:00Z'
updated_date: '2026-11-14T01:28:00Z'
labels: []
dependencies: []
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Harvesters got stuck in the middle or on the edge of a spice field and stopped working.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A harvester stopped next to spice, or full, gets back to work by itself
- [x] #2 A harvester looking for spice only goes for spice it can drive onto
- [x] #3 A harvester that can't reach where it roamed to picks again
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Idle harvesters go back to harvesting within 2 cells of spice, or empty when full; sand_near only picks drivable spice; roam reseeds by position.
<!-- SECTION:FINAL_SUMMARY:END -->
