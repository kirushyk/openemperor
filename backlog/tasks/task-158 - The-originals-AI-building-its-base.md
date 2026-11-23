---
id: TASK-158
title: 'The original''s AI: building its base'
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 158000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
CAiConstructionManager (AiConstructionManager.cpp, path at file 0x1e5684): building groups and ratios, its start script, critical buildings, turrets and walls, money split (messages 0x1e4858-0x1e5668). It reads ai.ini (file 0x1e6020, read at 0x3101b) or ai_%s.ini (0x1e6014): keys at 0x1e588c-0x1e5e40 (MaxAiUnits, NumBuildings, BuildingDelay, MinMoneyToStartBuildingWalls, MaxTurretsAllowed, ExtraPower...) and sections at 0x1e5f1c-0x1e5fbc (UnitLevel, Construction, Strategy, StartScript, PositionAlgorithmRatiosExits/NoExits, GeneralConstruction, UnitConstructionRatios, BuildingConstructionRatios). Walls give up after a while (0x1e4e6c, used at 0x2e667). Ours is our own. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The computer builds its base from ai.ini's start script, groups and ratios
- [ ] #2 It places buildings, turrets and walls as the original's position algorithms do
- [ ] #3 Tests check the first buildings it puts up on a skirmish map
<!-- AC:END -->
