---
id: TASK-149.8
title: Script function F99 RemoveMapShroud
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-149
ordinal: 237000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf34fc, 0xa bytes; +0x400000 in memory): RemoveMapShroud(): for each human side, uncovers the whole map (world shroud +0xa270, 0x4e6f60).  Used 5 times in 5 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's RemoveMapShroud does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls RemoveMapShroud and checks the effect
<!-- AC:END -->
