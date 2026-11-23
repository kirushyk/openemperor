---
id: TASK-159.27
title: Original's SpecifyObjects, SpecifyMainEnemySide and SetEncounterBehaviour
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 322000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ SpecifyObjects, SpecifyMainEnemySide and SetEncounterBehaviour, in GAME.EXE at file offset 0x52500, 0x20 bytes; file offset 0x52520, 0x20 bytes; file offset 0x52540, 0x20 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: The tactic base class's defaults: log that the tactic does not handle the call; each tactic overrides those it takes. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what SpecifyObjects, SpecifyMainEnemySide and SetEncounterBehaviour does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
