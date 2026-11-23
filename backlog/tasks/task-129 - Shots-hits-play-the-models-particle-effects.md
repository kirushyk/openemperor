---
id: TASK-129
title: Shots' hits play the models' particle effects
status: To Do
assignee: []
created_date: '2026-11-22T01:05:00Z'
updated_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 129000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Where a shot hits, its bullet's ExplosionType model (mghit, shellhit, missilehit, sniperhit, deviatehit, bloodsplat) is only boxes marking where its sparks fly; the sparks themselves are in the model's effects block, which we don't read yet. Read those effect definitions and play them at the hit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The effects block's particle definitions are read
- [ ] #2 Each shot's hit plays its ExplosionType model's effects where it hits
- [ ] #3 Tests cover it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
GAME.EXE (file offsets; +0x400000 for the address; map in docs/game-exe.md): see TASK-174: FXDataHeader (0x1c75c3), FXManager TriggerSound/TriggerWeapon/SetLight (0x1f603c-0x1f61dc, used at 0xb3bdf), 'Invalid FX header' (0x1f62bc, at 0xb4376).
<!-- SECTION:NOTES:END -->
