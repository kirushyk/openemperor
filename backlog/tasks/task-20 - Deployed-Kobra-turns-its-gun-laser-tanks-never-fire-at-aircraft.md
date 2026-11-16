---
id: TASK-20
title: Deployed Kobra turns its gun; laser tanks never fire at aircraft
status: Done
assignee: []
created_date: '2026-11-16T01:14:00Z'
updated_date: '2026-11-16T01:28:00Z'
labels: []
dependencies: []
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A deployed Kobra should turn its raised cannon towards its target. Laser tanks cannot attack aircraft.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A deployed Kobra turns its cannon to aim, as the original's
- [x] #2 A laser tank neither targets nor fires at aircraft
- [x] #3 Tests cover both
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Deployed Kobra: Unit::gun_turn swings its cannon round at the original's 4° an update (ORKobraDeployedGun), its body staying put; it must be round onto its aim, and raised, before firing. Its shots come from the swung cannon, on the disc's model (the gunpivot part, swung about the model's up) and on ours; packing up swings it back over the nose. Laser tanks: WeaponKind::Laser can only target the ground, as Rules' Laser_B has no AntiAircraft. Tests in weapons, units and a disc-backed one in render::original; older tests that had lasers hitting aircraft now say they cannot.
<!-- SECTION:FINAL_SUMMARY:END -->
