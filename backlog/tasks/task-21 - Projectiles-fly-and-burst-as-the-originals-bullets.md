---
id: TASK-21
title: Projectiles fly and burst as the original's bullets
status: Done
assignee: []
created_date: '2026-11-16T08:52:00Z'
updated_date: '2026-11-16T15:14:00Z'
labels: []
dependencies: []
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Missiles and shells take each bullet's Speed, TurnRate, HomingDelay, BlastRadius, ReduceDamageWithDistance and FriendlyDamageAmount from Rules.txt.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each missile flies at its bullet's Speed and turns at its TurnRate, after its HomingDelay
- [x] #2 Each blast reaches its bullet's BlastRadius, falls off only where ReduceDamageWithDistance allows, and hurts friends by FriendlyDamageAmount
- [x] #3 Tests cover them
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Bullet now carries the original's Speed and TurnRate (per update, ×25), HomingDelay, BlastRadius, ReduceDamageWithDistance and FriendlyDamageAmount for every missile and shell weapon. Missiles fly at their speed (HEAT 700/s, gunship 300/s, plasma 500/s), chase at their turn rate after their delay (the Ordos mine's rockets: straight for 5 updates, then 0.9 an update), and plasma, not homing, flies straight. Shells and missile hits burst through one function: blast from the bullet, falloff only where allowed, friends taking the friendly share (mortar, Kobra and gas shells hurt their own side by half). Rules' gunship rockets and plasma have no BlastRadius, so they no longer splash. Also fixed: a deployed Kobra reloads in its own 30 updates, not the Minotaurus's 120. Removed our replaced constants. Tests updated to the original; new ones for flight, homing delay, falloff and reloads.
<!-- SECTION:FINAL_SUMMARY:END -->
