---
id: TASK-149
title: Script shroud, worms, nukes, air strikes and reinforcement settings
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 149000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Not carried out: ReplaceShroud F98 (case file 0xf34d4, 0x28 bytes), RemoveMapShroud F99 (0xf34fc, 0xa), SideAttractsWorms F80 (0xf2a99, 0xa9; SideRepelsWorms F81 unused), ForceWormStrike F82 (0xf2c02, 0x1f), SideNukeAll F94 (0xf3469, 0x30; SideNuke F93 unused), AirStrike F153 and AirStrikeDone F154 (0xf425e, 0xf42bb) and SetReinforcements F92 (21 uses; 0xf2ea7, 0x21). Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ReplaceShroud covers again what was uncovered and RemoveMapShroud uncovers the whole map
- [ ] #2 ForceWormStrike and SideAttractsWorms bring worms as the original's case code does
- [ ] #3 AirStrike sends one and AirStrikeDone tells when it is over
- [ ] #4 SetReinforcements turns the side's reinforcements on or off
<!-- AC:END -->
