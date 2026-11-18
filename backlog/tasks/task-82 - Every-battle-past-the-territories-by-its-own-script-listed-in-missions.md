---
id: TASK-82
title: Every battle past the territories by its own script, listed in --missions
status: Done
assignee: []
created_date: '2026-11-18T23:44:00Z'
updated_date: '2026-11-18T23:50:00Z'
labels: []
dependencies: []
ordinal: 82000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User: 'i can not see in list of maps tutorial, end mission with worm, and some other mission', 'also list of --missions is not full'. The homeworld defences and assaults, the civil war's sides and the last battle ran with two MCVs; --missions listed only the first battles, territories and Heighliners.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each battle past the territories runs its house's script from the disc
- [x] #2 --missions lists them, names with spaces quoted
- [x] #3 None is decided before it has been fought
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
BATTLE_SCRIPTS names each house's script for the Heighliner, home defence, civil war sides, homeworld assaults and last battle; fight runs them; --missions lists all 156, quoting names with spaces. Three were won at once: the map's objects were put down after the script's first look (now before), F37 (a side's count of a kind) was unknown and read 0, and F83(FALSE) won instead of losing. K170/K172/K173 are >=, >, <=; without them a reinforcement block ran every update on Giedi Prime. All 16 now run 10 s undecided.
<!-- SECTION:FINAL_SUMMARY:END -->
