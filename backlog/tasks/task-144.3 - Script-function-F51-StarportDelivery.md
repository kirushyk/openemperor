---
id: TASK-144.3
title: Script function F51 StarportDelivery
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-144
ordinal: 200000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf0ff8, 0xf5 bytes; +0x400000 in memory): StarportDelivery(side, kind, kind, ...): unused by the disc's scripts; adds each listed kind to the side's starport order (Side 0x53bd00, 'Failed to find starport data entry') and sends the order (0x53c610), so the frigate brings them (co-op: through 0x4ee070).  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's StarportDelivery does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls StarportDelivery and checks the effect
<!-- AC:END -->
