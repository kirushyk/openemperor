---
id: TASK-126
title: Last mission's worm faces the way it goes
status: In Progress
assignee: []
created_date: '2026-11-21T23:05:00Z'
updated_date: '2026-11-21T23:17:00Z'
labels: []
dependencies: []
ordinal: 126000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The giant worm's head in the final mission does not look the way it should.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The worm's head looks the way the original's does
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Placed by each final mission with K139(..., GUWormhead, v18, -4, -1, 2): facing 2, now RightTop (head to -z, north); before the facing renumbering it looked +z (south). The model's head and jaws point to its front; its body bends through 201 frames we play. Stands at (7152, 2608) of an 8192x5120 map, the player's base far west. Waiting on which way the original's looks.
<!-- SECTION:NOTES:END -->
