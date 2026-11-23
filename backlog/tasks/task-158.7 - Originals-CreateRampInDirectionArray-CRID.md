---
id: TASK-158.7
title: Original's CreateRampInDirectionArray (CRID)
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 287000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CreateRampInDirectionArray (CRID), in GAME.EXE at file offset 0x2ca20, 0xad bytes (+0x400000 in memory; the size runs to the next known function start). What it does: For a side, finds the ramp off its rock in each direction ('CreateRampInDirectionArray: Finding ramps for %d %d', 'CRID: Dir: %d Ramp: %d %d' or 'No Ramp'), warning 'Unable to find ramps on rock section'. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CreateRampInDirectionArray (CRID) does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
