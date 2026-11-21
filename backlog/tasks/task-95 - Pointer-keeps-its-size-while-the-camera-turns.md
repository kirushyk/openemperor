---
id: TASK-95
title: Pointer keeps its size while the camera turns
status: Done
assignee: []
created_date: '2026-11-21T05:45:00Z'
updated_date: '2026-11-21T05:53:00Z'
labels: []
dependencies: []
ordinal: 95000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
When rotating the camera the mouse pointer grows to nearly the whole screen.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The pointer shown while turning the camera is its normal size
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
A cursor laid on the map was sized by how far a step east on the land moves on the screen; with the camera turned so that east runs into the screen that is next to nothing, and the cursor grew to fill it. It is now laid along the ways on the land that run across and down the screen, from both a step east and a step south, at any turn. Tested at eight turns.
<!-- SECTION:FINAL_SUMMARY:END -->
