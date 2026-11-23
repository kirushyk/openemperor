---
id: TASK-148.1
title: Script function F102 CameraScrollToPoint
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
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
- [ ] #1 The game does what the original's CameraScrollToPoint does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls CameraScrollToPoint and checks the effect
<!-- AC:END -->
