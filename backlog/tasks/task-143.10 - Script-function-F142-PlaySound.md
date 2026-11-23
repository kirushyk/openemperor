---
id: TASK-143.10
title: Script function F142 PlaySound
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-143
ordinal: 266000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3e9a, 0x15 bytes; +0x400000 in memory): PlaySound(n): unused; plays sound event n through the sound manager (0x808cfc, 0x4b4cc0).  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's PlaySound does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls PlaySound and checks the effect
<!-- AC:END -->
