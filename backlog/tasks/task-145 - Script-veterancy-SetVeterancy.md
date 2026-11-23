---
id: TASK-145
title: 'Script veterancy: SetVeterancy'
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 145000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
SetVeterancy F156 is called 1319 times in 51 scripts to give units their ranks; we ignore it. Case file 0xf4301, 0x42 bytes. Veterancy itself is done (TASK-85); its upgrade effect is VeteranUpgrade/VetLevelFX (strings file 0x1f6ffc and 0x1f700c, used at file 0xc3702 and 0xc36cc). Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 SetVeterancy sets a unit's rank as the original's case code does
- [ ] #2 A test checks a unit put down at a rank keeps its medals and extra figures
<!-- AC:END -->
