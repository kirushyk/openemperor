---
id: TASK-141
title: Script queries we don't answer yet
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 141000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Mission scripts ask these and we return nothing: Random F1 (55 uses in 31 scripts; case file 0xf2288, 0x21 bytes), GetEntranceNearToPos F10 (429/24; 0xf0776, 0x55), GetSecondPlayerSide F15 (23/4; 0xf0d95, 0xed), GetSideCash F19 (0xf1f6c, 0xa1), GetSideSpice F20 (0xf200d, 0xbe), ObjectNearToSideBase F25 (31/11; 0xf1920, 0xe5), ObjectTypeVisibleToSide F28 (0xf09f6, 0xd5), SideVisibleToSide F31 (61/32; 0xf0b32, 0x1ff), SideNearToSideBase F33 (40/17; 0xf1ca1, 0x205), SideNearToPoint F34 (91/54; 0xf1ea6, 0xc6), GetNeutralSide F140 (0xf3e84, 0x16), GetNeutralExitPoint F141 (0xf408a, 0x54). Uses are counted over the 228 .tok scripts of MISSIONS0001.RFD. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each listed function returns what the original's case code returns
- [ ] #2 Tests check each on a small script
- [ ] #3 No warning about these functions is logged running every campaign script
<!-- AC:END -->
