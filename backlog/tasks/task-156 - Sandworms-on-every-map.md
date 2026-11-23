---
id: TASK-156
title: Sandworms on every map
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 156000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's worms: the rules' MaximumSurfaceWorms, ChanceOfSurfaceWorm, ChanceOfVerticalWorm, SurfaceWormMinLife/MaxLife, SurfaceWormDisappearHealth, MinimumTicksWormCanAppear, WormAttractionRadius (file 0x21135c-0x2118d4), each kind's TastyToWorms and WormAttraction (0x21bf3c, 0x21c004), and the code: AiSurfaceWorm.cpp (0x1eade0), the vertical worm ('Vertical Worm: Finding target' 0x1eda04, used at 0x6cc57), CAiWormManager (0x1eda88, used at 0x6d9b2) with its WormSign message (0x1eda7c), unitWorm.cpp (0x222bc0, used from 0x171578) and the worm rider (unitWormRider.cpp 0x222be8; rules MinWormRideWaitDelay/MaxWormRideWaitDelay/WormRiderLifespan 0x2107a4-0x210ac4). TASK-87 is the tutorial's one scripted worm. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Worms surface and strike on sand as the rules' chances and figures say, drawn to what is tasty to them
- [ ] #2 The battlefield says wormsign when one comes
- [ ] #3 A Fremen worm rider rides one for its rules' time
<!-- AC:END -->
