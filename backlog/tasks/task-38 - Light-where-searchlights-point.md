---
id: TASK-38
title: Light where searchlights point
status: Done
assignee: []
created_date: '2026-11-17T11:30:00Z'
updated_date: '2026-11-17T11:30:00Z'
labels: []
dependencies: []
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A light source where the Mongoose's and other units' searchlights point, as the original.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each disc model's searchlight beam lights the ground at its end
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Part::beam marks a model's ?Spotlight parts; original::searchlight_spots gives each beam's far corner as posed at that moment. render::lights_of adds a pale PointLight there (and at the end of a deployed MCV's sweeping beam), the nearest the camera first, after fires and before gunfire. 40 disc models have beams. Tests: disc Mongoose lights ahead of itself as it turns; an MCV's beam lights the ground; older light tests leave searchlights out.
<!-- SECTION:FINAL_SUMMARY:END -->
