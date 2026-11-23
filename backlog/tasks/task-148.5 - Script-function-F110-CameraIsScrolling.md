---
id: TASK-148.5
title: Script function F110 CameraIsScrolling
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-148
ordinal: 242000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf36cb, 0x2a bytes; +0x400000 in memory): CameraIsScrolling(): unused; TRUE while a CameraScrollToPoint runs (+0x110). The main camera is game (0x808d14) +0x64 view, +0x6ec camera controller. No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's CameraIsScrolling does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls CameraIsScrolling and checks the effect
<!-- AC:END -->
