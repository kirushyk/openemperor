---
id: TASK-158.9
title: Original's CAiConstructionManager (save, load, queue)
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 289000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CAiConstructionManager (save, load, queue), in GAME.EXE at file offset 0x2ff20, 0x37 bytes; file offset 0x30042, 0xde bytes; file offset 0x30120, 0xd8 bytes; file offset 0x301f8, 0x1f8 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: The AI's base builder: keeps a building queue by groups (Critical, Resource, Manufacturing, Defence), its start script's steps, ratios and maintenance; saves and loads under the tag AiCM%d with its unit queue AiUC%d ('Number of building groups in save file mismatch'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CAiConstructionManager (save, load, queue) does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
