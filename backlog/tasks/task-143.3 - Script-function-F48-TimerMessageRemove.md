---
id: TASK-143.3
title: Script function F48 TimerMessageRemove
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-143
ordinal: 197000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf0299, 0x26 bytes; +0x400000 in memory): TimerMessageRemove(): destroys the script's timer box (0x4eb090) and clears script+0x390.  Used 9 times in 5 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's TimerMessageRemove does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls TimerMessageRemove and checks the effect
<!-- AC:END -->
