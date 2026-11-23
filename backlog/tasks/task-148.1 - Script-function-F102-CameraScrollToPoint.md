---
id: TASK-148.1
title: Script function F102 CameraScrollToPoint
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:28:00Z'
labels: []
dependencies: []
parent_task_id: TASK-148
ordinal: 238000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3586, 0x41 bytes; +0x400000 in memory): CameraScrollToPoint(pos, n): unused by the disc's scripts; stops a pan (+0x10c = 0), sets scrolling (+0x110 = 1) and slides the look-at point to pos over n updates (target +0x118, time +0x130, step from 0x4b8180), keeping angle and zoom. The main camera is game (0x808d14) +0x64 view, +0x6ec camera controller. No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's CameraScrollToPoint does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls CameraScrollToPoint and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
CameraScrollToPoint slides the camera at one pace (Camera::scroll_to, a glide that does not ease, replacing any pan); CameraIsPanning, CameraIsScrolling and CameraIsSpinning ask Camera::is_panning, is_scrolling and is_spinning. Tested in battlefield.rs (a_scripts_camera_scrolls_at_one_pace_and_says_what_it_is_doing) and mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
