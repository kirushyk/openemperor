---
id: TASK-78
title: Searchlights light the ground where they point
status: Done
assignee: []
created_date: '2026-11-18T18:28:00Z'
updated_date: '2026-11-20T23:56:00Z'
labels: []
dependencies: []
ordinal: 78000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User: 'searchlights in first atr mission should have light effect where they directed'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each searchlight in the Atreides first mission casts a pool of light on the ground where its beam points, moving with it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
original::searchlight_spots left out every building, so the GPINSpotlight props (buildings) of the Atreides' first battle lit nothing. A spotlight now gives the spot where its model's beam meets the ground, which lights_of makes a pool of white light that sweeps with it; other buildings' lights still light only themselves. Disc test.
<!-- SECTION:FINAL_SUMMARY:END -->
