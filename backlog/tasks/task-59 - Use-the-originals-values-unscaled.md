---
id: TASK-59
title: Use the original's values unscaled
status: Done
assignee: []
created_date: '2026-11-18T15:17:00Z'
updated_date: '2026-11-18T15:26:00Z'
labels: []
dependencies: []
ordinal: 59000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Health and damage are divided by HEALTH_SCALE (6) and blast radii stretched by 1.508; the user wants the rules' own values and proportions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Units' health and weapons' damage are the rules' figures, with no HEALTH_SCALE
- [x] #2 Blast radii are the rules' BlastRadius in the original's units
- [x] #3 Every other amount of health (repair, healing, burning, crushing, storms) keeps its effect at the new scale
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
HEALTH_SCALE removed (730a6fa): health and damage as in the rules; saboteur 96 and barrel 128 blast radii; constants the rules lack kept their effect (x6). Sight from the rules' ViewRange (first figure; the InfRock second is unknown).
<!-- SECTION:FINAL_SUMMARY:END -->
