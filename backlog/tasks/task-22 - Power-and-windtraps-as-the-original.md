---
id: TASK-22
title: Power and windtraps as the original
status: Done
assignee: []
created_date: '2026-11-16T15:20:00Z'
updated_date: '2026-11-16T15:37:00Z'
labels: []
dependencies: []
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Windtraps make power, buildings use it, as Rules.txt's PowerGenerated and PowerUsed; what needs a windtrap needs one; low power acts as the original.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Windtrap can be built and makes 100 power; each building uses Rules' PowerUsed
- [x] #2 Barracks, refinery, factory and outpost need a windtrap
- [x] #3 On low power the DisableWithLowPower turrets stop, and the sidebar shows power
- [x] #4 Opponents build windtraps to stay powered
- [x] #5 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Role::Windtrap (225 credits, 100 power, 2×2, disc models and pictures per house). Role::power_used from Rules; units::power, settle_power (advanced turrets go unpowered and hold fire), work_pace (half while short, our guess). Barracks, refinery, factory and outpost need a windtrap. The gauge round the minimap shows real power. Opponents build a windtrap first and another whenever the next building would overdraw. Their room search tries more places on wider rings, as windtraps crowd a base. Test bases have windtraps.
<!-- SECTION:FINAL_SUMMARY:END -->
