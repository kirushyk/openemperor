---
id: TASK-144.1
title: Script function F49 CarryAllDelivery
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-144
ordinal: 198000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf0f38, 0x59 bytes; +0x400000 in memory): CarryAllDelivery(side, kind, pos): returns an obj. Looks up the kind named "Carryall" (0x5974c0) and has the world fly one in (0x594580(pos, kind, carryall kind, side, 1)) carrying a new `kind` for `side` to pos; the result is the delivered object's handle.  Used 45 times in 11 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's CarryAllDelivery does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls CarryAllDelivery and checks the effect
<!-- AC:END -->
