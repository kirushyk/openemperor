---
id: TASK-158.12
title: Original's ChooseClusterWithDefencePlan
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 292000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ ChooseClusterWithDefencePlan, in GAME.EXE at file offset 0x2e514, 0xdc bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Picks the base cluster that needs defending and makes a defence plan for it if it has none ('Creating new defence plan for cluster'; 'Defence plan has no points, disabling'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what ChooseClusterWithDefencePlan does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
