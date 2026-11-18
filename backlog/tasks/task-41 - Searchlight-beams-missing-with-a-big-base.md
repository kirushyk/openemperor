---
id: TASK-41
title: Searchlight beams missing with a big base
status: Done
assignee: []
created_date: '2026-11-18T11:25:00Z'
updated_date: '2026-11-18T11:25:00Z'
labels: []
dependencies: []
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Glows drawn on worker threads were dropped when the canvases were merged, so with 24 or more units every beam and windtrap wind vanished
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Decals drawn on threads are merged back
- [x] #2 A test draws glows on threads and keeps them all
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
6c6fab4: draw_on_threads merges decals.
<!-- SECTION:FINAL_SUMMARY:END -->
