---
id: TASK-142.1
title: Script function F39 EventObjectDelivered
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-142
ordinal: 191000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf2eda, 0x92 bytes; +0x400000 in memory): EventObjectDelivered(var, side): TRUE if this update's list has a kind-8 event whose object belongs to `side` (0x4f5a80); the delivered object goes into var. Each script keeps a linked list of this update's events at script+0x2c0: records {kind @0, object handle @4, value @8, next @0x1c}. Kind 8 = an object delivered, kinds 6 and 9 = an object constructed (built or placed), kind 11 = an object attacked a side (attacker handle @4, side @8). An Event function's first argument must be an obj variable (0x81 token, slot kind 2, else Script::Error); on a match it stores the event object's handle in that variable and returns TRUE, otherwise FALSE. Used 2 times in 2 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's EventObjectDelivered does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls EventObjectDelivered and checks the effect
<!-- AC:END -->
