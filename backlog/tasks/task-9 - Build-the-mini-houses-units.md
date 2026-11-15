---
id: TASK-9
title: Build the mini-houses' units
status: Done
assignee: []
created_date: '2026-11-14T01:28:00Z'
updated_date: '2026-11-14T05:29:00Z'
labels: []
dependencies: []
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The units of an allied mini-house can be made, as in the original.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 They cost, take as long and fight as Rules.txt says, and draw with the disc's models
- [x] #2 Units the game does not have yet are added, and docs/units.md is brought up to date
- [x] #3 Tests cover when each shows in the menu and that it can be made
- [x] #4 Each allied mini-house's units that need no upgraded building are in the build menu, made where Rules.txt says and needing what it says; those that do are moved to TASK-16
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added each mini-house's first unit: Role::TlContaminator, IxInfiltrator, GuMaker, ImSardaukar and FrFremen (display names from the disc). Ally::unit maps them. They are made in their mini-house's main building (made_in; those buildings now make_units, exiting like any other) and sit in Infantry cells 5 and 6 for the two ally slots, since the Vehicles menu is full. Cost and speed are Rules.txt's; health and build time follow our scaling. Weapons from their Rules guns: Fremen the sniper's cal50 and stealthed; Sardaukar heavy MG as Bullet; Maker howitzer as Mortar; Contaminator a short stab as Pistol (no contamination yet); Infiltrator stealthed, unarmed, and deploying sets off the saboteur's bomb (Resource = SaboteurBomb). Disc models units\*_H0.xbf load; selection boxes measured from their SLCT nodes. Without the disc, our APC and Inkvine stand in. The upgraded units (Projector, Leech, ADV Sardaukar, ADV Fremen, NIAB Tank) need building upgrades, which we lack: moved to new TASK-16. IMDropShip isn't built. docs/units.md updated. Tests: menu cells and making each one in its building; the infiltrator's bomb and stealth, the Fremen's stealth and the weapons.
<!-- SECTION:FINAL_SUMMARY:END -->
