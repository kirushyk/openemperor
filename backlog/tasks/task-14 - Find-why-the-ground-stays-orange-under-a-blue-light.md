---
id: TASK-14
title: Find why the ground stays orange under a blue light
status: In Progress
assignee: []
created_date: '2026-11-14T01:28:00Z'
updated_date: '2026-11-21T00:03:00Z'
labels: []
dependencies: []
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
On maps with a dark blue ambient and directional light, buildings take the hue but the ground stays orange.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 On maps with a blue light, such as Spice Bowl, the ground takes the light's hue the way buildings do
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-21: Found: the wash (test.CPT) is the colors of the map's sand and rock with no light in it (seen dumped from #u1: no shading, no shadows), but the ground shader laid it on unlit, so the map's light never reached it. Now the ground's color and its wash are blended first and then lit by the map's ground light (test.lit lines 2 and 3), as the rest of the ground is. Not seen yet: awaiting the user's look.
<!-- SECTION:NOTES:END -->
