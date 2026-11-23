---
id: TASK-148.3
title: Script function F104 CameraViewFrom
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-148
ordinal: 240000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf35ef, 0x28 bytes; +0x400000 in memory): CameraViewFrom(a, n): unused by the disc's scripts; sets +0x113 = 1, +0x111 = 0, +0x13c = n, +0x140 = 1 and +0x138 = 2 (the first argument is not read): a move to a fixed view taking n updates. The main camera is game (0x808d14) +0x64 view, +0x6ec camera controller. No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's CameraViewFrom does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls CameraViewFrom and checks the effect
<!-- AC:END -->
