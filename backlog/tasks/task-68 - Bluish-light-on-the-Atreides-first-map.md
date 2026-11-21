---
id: TASK-68
title: Bluish light on the Atreides' first map
status: In Progress
assignee: []
created_date: '2026-11-18T16:25:00Z'
updated_date: '2026-11-21T00:03:00Z'
labels: []
dependencies: []
ordinal: 68000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The first Atreides mission's ground should take the bluish light the units have.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The first Atreides map's ground is lit in the same bluish tones as its units
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-21: Found: the wash (test.CPT) is the colors of the map's sand and rock with no light in it (seen dumped from #u1: no shading, no shadows), but the ground shader laid it on unlit, so the map's light never reached it. Now the ground's color and its wash are blended first and then lit by the map's ground light (test.lit lines 2 and 3), as the rest of the ground is. Not seen yet: awaiting the user's look.
<!-- SECTION:NOTES:END -->
