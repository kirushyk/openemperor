---
id: TASK-81
title: Know every unit the maps and scripts put down, the yak among them
status: To Do
assignee: []
created_date: '2026-11-18T19:10:00Z'
updated_date: '2026-11-18T19:21:00Z'
labels: []
dependencies: []
ordinal: 81000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User: 'add yak and also check and add all other units'. Map objects and script kinds with no role are skipped.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every unit or building kind a map's objects or a mission script puts down is known as a role, with the rules' values and the disc's model
- [x] #2 INYak, INYakHauder and INYakRider stand on the Harkonnen first map
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Stage 1: 50 kinds as Role::Extra from a table generated off Rules.txt and ArtIni.txt, each with the disc's model. Weapons only where an existing one is the same bullet (militia, Atreides general, projector, quad, Tleilaxu turret). Left: the special weapons (sonic, deviator, leech, AA trooper, elite Sardaukar, NIAB, NIAP, mega cannons, slave knife); kind #279 of Save The Duke, past the rules' lists; LeechCrate (a crate).
<!-- SECTION:NOTES:END -->
