---
id: TASK-8
title: Build the mini-houses' main buildings
status: Done
assignee: []
created_date: '2026-11-14T01:28:00Z'
updated_date: '2026-11-14T05:05:00Z'
labels: []
dependencies: []
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The player can put up the main building of each mini-house the player is allied with, as in the original.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each mini-house's main building from Rules.txt is in the build menu once the player is allied with it, and not before
- [x] #2 It costs, takes as long and needs what Rules.txt says, and draws with the disc's model
- [x] #3 Tests cover when it shows in the menu and that it can be built and placed
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added Role::TlFleshVat, IxResCentre, GuPalace, ImBarracks and FrCamp (Rules names; display names from the disc's strings). Ally::main_building maps each mini-house to its building. In a skirmish the Buildings menu offers the player's two allies' buildings in cells 9 and 10 (Game::player_allies), and nothing when not allied. Costs (1000, 1000, 2000, 700, 600) and footprints (3×6, 7×7, 7×13, 3×5, 6×6, without the s rows) and roof heights are Rules.txt's. Role::needs adds Rules' SecondaryBuilding: a factory for Tleilaxu, Ix and Guild, a barracks for Sardaukar and Fremen. can_produce checks it and the tooltip says what is missing. Build times and health follow our existing scaling: 20 s like the factory, 30 s like the palace. They draw with the disc's buildings\*_H0.xbf models and their damaged variants (all five load); without the disc, the plain block. docs/units.md updated. Tests: menu cells and needs; building, paying and placing each one.
<!-- SECTION:FINAL_SUMMARY:END -->
