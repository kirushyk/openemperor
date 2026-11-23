---
id: TASK-158.1
title: Original's AddNewOwnedBuilding
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 281000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ AddNewOwnedBuilding, in GAME.EXE at file offset 0x2dd60, 0x120 bytes; file offset 0x2ecf5, 0x16b bytes (+0x400000 in memory; the size runs to the next known function start). What it does: CAiConstructionManager: when the side gets a building, rejects one with no health ('trying to add building with no health!'), adds it to its count by type ('Side %d New building added,type: %d (%s)', 'could not increase building count for type %d') and puts it in a base cluster, creating a new cluster if it is in none ('New building is not in any existing cluster, creating new cluster'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what AddNewOwnedBuilding does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
