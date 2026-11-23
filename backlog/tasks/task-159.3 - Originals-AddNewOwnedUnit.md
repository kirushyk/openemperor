---
id: TASK-159.3
title: Original's AddNewOwnedUnit
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 298000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ AddNewOwnedUnit, in GAME.EXE at file offset 0x5a430, 0x110 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: CAiTacticsManager: when the side gets a unit (never a dead one), records its type and sorts it into the AI's unit classes: standard, deployable, engineer, scout, move-and-fire, crusher, line of fire, carrier, saboteur, EITS, ornithopter, carryall, deviator, devastator, IX projector, Fremen, Leech, contaminator, Sardaukar, NIAB, infiltrator, ADP ('Identified ... unit %s'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what AddNewOwnedUnit does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
