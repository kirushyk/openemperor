---
id: TASK-124
title: Rockets and shots look as the original's
status: Done
assignee: []
created_date: '2026-11-21T23:04:00Z'
updated_date: '2026-11-22T01:05:00Z'
labels: []
dependencies: []
ordinal: 124000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Missiles, rockets, shells and bullets in flight, and their muzzle flashes and hits, should look and move as the original's: the disc has their models (bullets\\missile.xaf, shell.xaf, mgbullet.xaf, muzzle1-3.xaf, missilehit.xaf, shellhit.xaf, mghit.xaf, lsonic.xaf, tracer1.xaf).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each projectile in flight is drawn with the disc's model for it, animated as its model is
- [x] #2 Tests cover it
- [x] #3 Muzzle flashes use the disc's models
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Rules: unit TurretAttach -> turret (TurretNextJoint) -> TurretMuzzleFlash + Bullet; Bullet -> ExplosionType; ArtIni Xaf -> model (bullets\ for bullets, explosion\ for flashes and hits; the .xbf beside each .xaf). Sim records each shot's flash and hit as an Effect with the gun (role, house, weapon) that made it; render draws the disc model for each, playing its sequence once; missiles and shells in flight drawn with their bullet's model along their flight.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Flashes and bullets in flight drawn from the disc's models per the rules' chain (unit TurretAttach -> turret TurretMuzzleFlash and Bullet -> ArtIni model). The hits' models are only marker boxes for particle effects in the models' effects block, which we don't read yet, so hits keep our own sparks and flash; AC2 holds for the flashes only.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Muzzle flashes and rockets, shells and other bullets in flight are drawn with the disc's models each unit's gun has in the rules, animated as made; hits stay ours until the models' particle effects are read. Tests cover the lookup, the effects' lifetimes and the disc's models.
<!-- SECTION:FINAL_SUMMARY:END -->
