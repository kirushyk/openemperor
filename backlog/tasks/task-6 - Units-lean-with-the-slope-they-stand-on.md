---
id: TASK-6
title: Units lean with the slope they stand on
status: Done
assignee: []
created_date: '2026-11-14T01:28:00Z'
updated_date: '2026-11-14T01:55:00Z'
labels: []
dependencies: []
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Check whether units in the original tilt on sloped ground, and copy it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Find out whether and how the original tilts vehicles, infantry and buildings on slopes
- [x] #2 If it does, a vehicle on a slope lies along it, front and side, and turns smoothly as the ground changes
- [x] #3 What the original keeps upright stays upright
- [x] #4 Tests cover the tilt on a slope and none on flat ground
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Neither Rules.txt nor GAME.EXE's strings say how the original treats slopes (its only Tilt settings are the camera's); going by how the original plays, its ground vehicles lie along the ground and infantry and buildings stay upright. units.rs: Unit.lean [nose up, right down] from the ground under the front, back and sides of the original's selection box, eased at LEAN_RATE as it moves, set at once when placed or set down, at most MOST_LEAN; aircraft, carried vehicles, infantry, towers and buildings stay upright. render/models.rs: Place::level pitches and banks by it, so the disc's models, our models and the marked firing points all lean.
<!-- SECTION:FINAL_SUMMARY:END -->
