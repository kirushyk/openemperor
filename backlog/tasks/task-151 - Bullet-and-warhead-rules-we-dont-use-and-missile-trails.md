---
id: TASK-151
title: Bullet and warhead rules we don't use, and missile trails
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 151000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
[BulletTypes] and [WarheadTypes] keys in GAME.EXE's rules table (file 0x20e79c, 0xc8-byte records) that we ignore: Lifespan (0x21342c), Continuous (0x2134f4), LingerDuration/LingerDamage (0x2135bc, 0x213684), Beserk (0x213814), Trajectory (0x2139a4), DamageFriendly (0x213b34), Ignites (0x213cc4), IsLaser (0x213d8c), AntiAircraft/AntiGround (0x213044, 0x21310c), DamageColumn (0x2131d4), DamageEffect/ChaosEffect/HawkEffect (0x213f1c-0x2140ac), Debris (0x214174), BlowUp (0x2143cc), HitSlowdownAmount/Duration (0x21c964, 0x21ca2c), AirborneMine (0x21cbbc), DamageToTile (0x21c0cc), and the trails MissileTrail, ...WiggleFreq, ...WiggleScale, ...Length, ...Size, ...Subdiv, ...Delta (0x21df44-0x21e3f4), drawn with systemmissiletrail_32.tga (file 0x1fdc14, used at 0xeb28b). Relates to TASK-19 and TASK-71. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each listed key's figure is used as its name says, or docs/not-implemented.md says why not
- [ ] #2 Missiles leave the original's trails
- [ ] #3 Lingering gas and fire hurt for LingerDuration at LingerDamage
<!-- AC:END -->
