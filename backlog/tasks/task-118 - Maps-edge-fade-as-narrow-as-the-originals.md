---
id: TASK-118
title: Map's edge fade as narrow as the original's
status: Done
assignee: []
created_date: '2026-11-21T21:32:00Z'
updated_date: '2026-11-21T21:33:00Z'
labels: []
dependencies: []
ordinal: 118000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The 4-square fade to black at the map's edges is too long.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The land fades to black over the last square to each edge
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
EDGE_FADE is one square, down from four; still a guess, as camera.ini, mapinfo.ini and the maps' files give no figure. The existing test holds at the new width.
<!-- SECTION:FINAL_SUMMARY:END -->
