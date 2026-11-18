---
id: TASK-48
title: Explosive barrels
status: Done
assignee: []
created_date: '2026-11-18T11:58:00Z'
updated_date: '2026-11-18T12:51:00Z'
labels: []
dependencies: []
ordinal: 48000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
INBarrel1/INBarrel2 (Rules: Health 10, Light armour, ExplosionType BarrelBomb) are placed by ATStart.tok and by the map, but we have no such object
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Barrels stand where the script and the map put them, with the disc's models
- [x] #2 A barrel shot to pieces explodes and hurts what is near
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
673e095: Role::Prop barrels with the disc's models; a destroyed one blows up as BarrelBomb (5000, all armour alike, radius 128 original units) and sets off its neighbours.
<!-- SECTION:FINAL_SUMMARY:END -->
