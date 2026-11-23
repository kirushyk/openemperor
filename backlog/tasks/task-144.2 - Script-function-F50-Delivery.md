---
id: TASK-144.2
title: Script function F50 Delivery
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-144
ordinal: 199000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf0f91, 0x67 bytes; +0x400000 in memory): Delivery(side, pos, kind, kind, ...): in co-op the human side becomes a random human partner. Core 0x4ee0c0: the world makes a starport frigate for the side at pos (0x597a30, which names "Frigate"), clears bit 0 of its flags (+0x2c8) and loads each listed kind into it (0x5694f0) - the frigate lands and unloads them.  Used 24 times in 11 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's Delivery does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls Delivery and checks the effect
<!-- AC:END -->
