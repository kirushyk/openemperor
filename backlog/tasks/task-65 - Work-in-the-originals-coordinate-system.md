---
id: TASK-65
title: Work in the original's coordinate system
status: To Do
assignee: []
created_date: '2026-11-18T16:02:00Z'
labels: []
dependencies: []
ordinal: 65000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user wants the original's coordinate system kept as far as possible. Models go through to_place ([-z, y, x]), the sidebar's symbols and cursors flip x, and scripted facings needed a 180-degree renumbering. Find each conversion and remove what can go.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every axis swap, mirror or turn between the disc's coordinates and ours is listed with why it is there
- [ ] #2 Those that are not needed are removed, with the game looking the same
<!-- AC:END -->
