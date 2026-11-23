---
id: TASK-158.2
title: Original's AiIni (ai.ini)
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 282000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ AiIni (ai.ini), in GAME.EXE at file offset 0x31720, 0x277 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Loads ai.ini, or ai_%s.ini for a side (0x1e6014), for each AI side ('AiIni: Attempting to load ini file <%s> for side %d'): its sections UnitLevel, Construction, Strategy, StartScript, PositionAlgorithmRatiosExits/NoExits, GeneralConstruction, UnitConstructionRatios, BuildingConstructionRatios and keys from MaxAiUnits to TicksToChangeStaging (0x1e588c-0x1e5e40), reporting bad lines ('AI FILE: Invalid line %d, label %s'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what AiIni (ai.ini) does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
