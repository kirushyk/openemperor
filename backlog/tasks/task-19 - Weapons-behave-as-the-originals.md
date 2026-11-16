---
id: TASK-19
title: Weapons behave as the original's
status: In Progress
assignee: []
created_date: '2026-11-16T01:04:00Z'
updated_date: '2026-11-16T05:01:00Z'
labels: []
dependencies: []
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Every weapon we have behaves as in the original GAME.EXE and Rules.txt: damage, range, reload, projectile speed and path, area, warheads against armour, and their special behaviours.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each weapon's numbers come from Rules.txt as the original reads them
- [ ] #2 Behaviour coded in the exe is followed where found, and what isn't is logged in docs/not-implemented.md
- [ ] #3 Tests cover the weapons' numbers and behaviours
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Step 1 done (63767f6): Rules health and armour per role, bullet damage, warhead shares and reloads per turret, saboteur/infiltrator/EITS/Ordos ADP bombs. Left, as logged in docs/not-implemented.md: salvo and spray rhythms, gas/poison/saw/superweapon numbers, blast radii and falloff, friendly damage, projectile speeds and homing, InfRock, shields, repair rate, per-house health, nothing read from GAME.EXE yet.
<!-- SECTION:NOTES:END -->
