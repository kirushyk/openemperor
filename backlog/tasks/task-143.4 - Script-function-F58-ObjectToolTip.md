---
id: TASK-143.4
title: Script function F58 ObjectToolTip
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-143
ordinal: 205000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3300, 0x2f bytes; +0x400000 in memory): ObjectToolTip(obj, n): calls the object's virtual function +0x1a8 with n, which sets the text the cursor shows over it.  Used 273 times in 30 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's ObjectToolTip does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls ObjectToolTip and checks the effect
<!-- AC:END -->
