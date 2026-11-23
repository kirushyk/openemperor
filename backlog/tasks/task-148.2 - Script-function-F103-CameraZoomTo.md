---
id: TASK-148.2
title: Script function F103 CameraZoomTo
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:29:00Z'
labels: []
dependencies: []
parent_task_id: TASK-148
ordinal: 239000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf35c7, 0x28 bytes; +0x400000 in memory): CameraZoomTo(percent, n): unused by the disc's scripts; zooms to `percent` of the zoom range between MinZoom and MaxZoom (+0x34, +0x30) over n updates: rate = ((current - min)/(max - min) x 100 - percent) x (max - min) x 0.01 / n (+0x148), flag +0x112. The main camera is game (0x808d14) +0x64 view, +0x6ec camera controller. No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's CameraZoomTo does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls CameraZoomTo and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
CameraZoomTo(percent, n) zooms the camera at one rate over n updates to MIN_ZOOM + percent% of (MAX_ZOOM - MIN_ZOOM), CAMERA.INI's MinZoom and MaxZoom (Camera::zoom_to). Tested in battlefield.rs (a_scripts_camera_zooms_at_one_rate_to_a_share_of_its_range) and mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
