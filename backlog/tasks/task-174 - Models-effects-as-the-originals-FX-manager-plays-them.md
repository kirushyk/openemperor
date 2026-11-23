---
id: TASK-174
title: Models' effects as the original's FX manager plays them
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 174000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The FX data of a model (FXDataHeader, file 0x1c75c3; 'Invalid FX header' 0x1f62bc used at 0xb4376; 'Version 2 of FX Data not supported' 0x1f6290) is played by the FX manager (FXManager->TriggerSound 0x1f603c at 0xb3bdf, TriggerWeapon, SpeedChange, SetLight 0x1f60b0-0x1f61dc). A building's states are named animations: Normal State, Damage State 1, Damage State 2, Construction Anim, Explode Anim (0x1f6214-0x1f6254), and moving plays MoveFxStart/MoveFx/MoveFxEnd (0x21f310-0x21f324, used at 0x12b11e). Relates to TASK-129 and TASK-70. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A model's effects (smoke, flashes, lights, sounds) play as its FX data says
- [ ] #2 Buildings play their construction, damage and explode animations
<!-- AC:END -->
