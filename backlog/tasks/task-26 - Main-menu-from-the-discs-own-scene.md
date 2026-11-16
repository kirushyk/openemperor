---
id: TASK-26
title: Main menu from the disc's own scene
status: Done
assignee: []
created_date: '2026-11-16T21:33:00Z'
updated_date: '2026-11-16T21:33:00Z'
labels: []
dependencies: []
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Draw the original's frontend main.xbf behind the main and single player menus, with our buttons where its are.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 With the disc, the main menus show its scene and title; without, ours
- [x] #2 What is left is logged
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
render/frontend.rs reads frontend\main.xbf from the interface archive with the models: its pictures cut into 128-pixel tiles (up to 4×4) and added to the backend's, its faces kept where they stand, moving parts (the planet) with their animation. Drawn flat each frame from the origin looking along +z, farthest first, back faces left out, triangles cut along tile edges; % and ! pictures add light. Main and single player menu buttons moved to the scene's own rows; our title is hidden over the scene's. Checked by rasterising the triangles to a picture. The cloud object is left out (blocky, unexplained) and other screens' scenes are logged.
<!-- SECTION:FINAL_SUMMARY:END -->
