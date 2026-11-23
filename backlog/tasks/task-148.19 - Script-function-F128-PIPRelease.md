---
id: TASK-148.19
title: Script function F128 PIPRelease
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-148
ordinal: 256000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3a23, 0x38 bytes; +0x400000 in memory): PIPRelease(): if a PIP window exists (0x578670), closes it (0x514c30) with 'GreenTVScreenOff'. Picture-in-picture: the view's +0x6f0, created on first use by any PIP function (0x5785c0): a window (0x5148e0) that draws a second camera (its own controller at +0x10, made from CAMERA.INI, 0x531c20) into a render texture, framed with 'reticule_64.tga', playing the interface event 'GreenTVScreenOn' as it opens; on creation its camera copies the main camera (0x5332b0) at zoom 0.6 (0x5324f0(0.6)). Each PIP function calls the same camera-controller method as its main-camera twin on the window's camera; PIPRelease closes the window ('GreenTVScreenOff'). Used 466 times in 155 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's PIPRelease does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls PIPRelease and checks the effect
<!-- AC:END -->
