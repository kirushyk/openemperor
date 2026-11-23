---
id: TASK-144
title: >-
  Script objects: deliveries, building, changing sides, infecting, detonating,
  deploying
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 144000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Not carried out: CarryAllDelivery F49 (45 uses in 11 scripts; case file 0xf0f38, 0x59 bytes), Delivery F50 (24/11; 0xf0f91, 0x67), BuildObject F52 (60/20; 0xf10ed, 0x65), ObjectChangeSide F53 (195/33; 0xf11c5, 0x33), ObjectInfect F55 (272/15; 0xf320b, 0x3d), ObjectDetonate F56 (49/11; 0xf3248, 0x65), ObjectDeploy F133 (0xf3aa2, 0x88), NewObjectInAPC F137 (60/5; 0xf3bb1, 0x95), SetTilePos F158 (67/5; 0xf438a, 0x7d), and the crates NewCrateUnit F86 (0xf332f, 0x58) and NewCrateCash F89 (41/7; one case for F87-F90 at 0xf3387, 0x6e). StarportDelivery F51, ObjectUndeploy F134, ObjectRemove F136 and ObjectIsCarried F138 are in the original but no script on the disc uses them. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each listed function does what the original's case code does
- [ ] #2 A carryall delivery is flown in by a carryall and a plain delivery appears at its entrance
- [ ] #3 Tests cover each function
<!-- AC:END -->
