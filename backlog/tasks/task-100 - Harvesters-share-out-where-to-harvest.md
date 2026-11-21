---
id: TASK-100
title: Harvesters share out where to harvest
status: Done
assignee: []
created_date: '2026-11-21T17:20:00Z'
updated_date: '2026-11-21T17:35:00Z'
labels: []
dependencies: []
ordinal: 100000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Two or three harvesters sometimes go for the same spot and circle it forever. They should agree among themselves where each harvests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No two harvesters go for the same spot of spice
- [x] #2 Harvesters sent to one field all get to harvesting, none circling
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Harvesters now claim the spot they head for and the one they stand on. Picking where to go next, out to the sand or roaming on it, a harvester takes the nearest spice no other has claimed, and only shares a spot when all are. On its way, one making for where another nearer (or as near and first) is making for too gives way and picks its own. Tested: three harvesters sent to one spot each harvest at their own, where before all three were still circling after 20 seconds.
<!-- SECTION:FINAL_SUMMARY:END -->
