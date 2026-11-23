---
id: TASK-16
title: Upgrade buildings, and make the mini-houses' upgraded units
status: To Do
assignee: []
created_date: '2026-11-14T05:07:00Z'
updated_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rules.txt gives buildings an UpgradeCost and UpgradeTechLevel. The mini-houses' Projector, Leech, ADV Sardaukar, ADV Fremen and NIAB Tank need an upgraded main building (UpgradedPrimaryRequired). Our game has no upgrades yet. The IMDropShip is called in, not built.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A building can be upgraded for its UpgradeCost, as in the original
- [ ] #2 IXProjector, TLLeech, IMADVSardaukar, FRADVFremen and GUNIABTank can be made once their mini-house's main building is upgraded, and not before
- [ ] #3 Tests cover upgrading and the units it allows
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
GAME.EXE (file offsets; +0x400000 for the address; map in docs/game-exe.md): the upgraded units' own code: UnitProjector.CPP (path 0x222b2c, used at 0x17024f) with ReplicaSpawn (0x222b84, at 0x170c67) and the rules' ReplicaFlickerChanceWhenMoving/WhenStill, ReplicaProjectionTime, ReplicaVanishTime, ReplicaShouldFire, ReplicaBulletDamage (0x21b514-0x21b8fc); unitNiabTank.cpp (0x222ad8, used from 0x16e007) with TeleportSleepTime (0x217f2c); unitAdvFremen.cpp (0x22270c, at 0x16627f); the Leech's mark {LEECH} (0x2030f4) and LeechMaxRangeToLookForUnits (0x1e59f4); IMADVSardaukar's AI AiUnitIMADVSardauker.cpp (0x1ed634).
<!-- SECTION:NOTES:END -->
