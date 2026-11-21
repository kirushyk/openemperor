---
id: TASK-104
title: Hourglass cursor while the script has control
status: Done
assignee: []
created_date: '2026-11-21T18:56:00Z'
updated_date: '2026-11-21T19:21:00Z'
labels: []
dependencies: []
ordinal: 104000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's cursors include cu_wait_h0; show it while a scene of the script plays.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 During a scene the cursor is the disc's hourglass
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
CursorLook::Wait, drawn with the disc's CU_Wait_H0 (an hourglass that turns over every two seconds), is the cursor while scene_playing; without the disc, the arrow. Clicks are not blocked during a scene. Tests: the look during a scene, and the disc's hourglass drawn about the pointer and turning over.
<!-- SECTION:FINAL_SUMMARY:END -->
