---
id: TASK-159.24
title: Original's ShouldThreatBeReactedTo
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 319000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ ShouldThreatBeReactedTo, in GAME.EXE at file offset 0x5eb4f, 0x201 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Whether the AI answers a threat: yes if it is near the base, near one of its harvesters, or near a strategic point ('Threat is near to base', 'near to harvester', 'near to strategic point %s'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what ShouldThreatBeReactedTo does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
