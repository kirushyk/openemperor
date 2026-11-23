---
id: TASK-143.1
title: Script function F46 GiftingMessage
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-143
ordinal: 195000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf01cd, 0x33 bytes; +0x400000 in memory): GiftingMessage(): shows the string 'Gifthint' of section 'sidebarhints' (string table 0xb6f198, 0x546d50/0x546c10) as an in-game message (0x4ea790 with 1,0,1).  Used 3 times in 3 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's GiftingMessage does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls GiftingMessage and checks the effect
<!-- AC:END -->
