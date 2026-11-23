---
id: TASK-159.5
title: Original's IsTypeSubhouseObject
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 300000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ IsTypeSubhouseObject, in GAME.EXE at file offset 0x3bfd0, 0x60 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Whether a kind belongs to a sub-house (Fremen, Sardaukar, Ix, Tleilaxu, Guild), by its rules' house ('Unknown house type %d'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what IsTypeSubhouseObject does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
