---
id: TASK-144.11
title: Script function F89 NewCrateCash
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-144
ordinal: 229000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3387, 0x6e bytes; +0x400000 in memory): One case for all four (0xf3387), told apart by the token number: makes a "crate" object at pos and sets its crate type: Cash 0, Stealth 1, Bomb 3, Shroud 4 (Unit is 2, F86); health -1. Takes only the position: what a cash crate pays is not given by the script.  Used 41 times in 7 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's NewCrateCash does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls NewCrateCash and checks the effect
<!-- AC:END -->
