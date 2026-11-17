---
id: TASK-36
title: The battlefield's spoken messages
status: Done
assignee: []
created_date: '2026-11-17T09:52:00Z'
updated_date: '2026-11-17T09:52:00Z'
labels: []
dependencies: []
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Say the original's UI lines from DIALOG.BAG when things happen on the battlefield.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each event we have says its line from UI_Comp.txt
- [x] #2 Lines are not repeated at once
- [x] #3 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
game::Speech for 21 events, raised by Game::announce (3 s quiet, 20 s for attacks and low power) from production, building, selling, losses, guard, routes, retreat, superweapon, outcome, and listen_to_the_base (health drops of buildings and harvesters, buildings finished, power running short). sounds::spoken reads INSTALL/DATA/DIALOG/DIALOG.BAG and maps each to its UI-G line from UI_Comp.txt; main plays them with play_line. Disc test checks every line exists.
<!-- SECTION:FINAL_SUMMARY:END -->
