---
id: TASK-152
title: Turrets' joints from the rules
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 152000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The rules' [TurretTypes] and [TurretJointTypes] give each turret's joints, limits and aim: TurretHowMany (file 0x2147b4), TurretMin/MaxX/YRotation (0x21487c-0x214ad4), TurretX/YRotationAngle (0x214b9c, 0x214c64), TurretX/YAcceptableAim (0x214d2c, 0x214df4), TurretNextJoint (0x214ebc), TurretAttach (0x214f84), TurretDisableIfUnitDeployed/Undeployed (0x21504c, 0x215114), TurretBulletCount (0x2151dc). The turret code is TurretControl.CPP (path at file 0x221b00). Ours turns guns by our own rule; the deployed Kobra's ±190° and -90°..30° limits are in docs/not-implemented.md. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every turret turns and raises within its rules' limits, at its rules' angles an update
- [ ] #2 A turret fires only once within its acceptable aim
- [ ] #3 Turrets disabled when deployed or undeployed are so
<!-- AC:END -->
