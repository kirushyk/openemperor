---
id: TASK-24
title: Upgrade buildings, and require upgrades as the original
status: Done
assignee: []
created_date: '2026-11-16T15:20:00Z'
updated_date: '2026-11-16T15:37:00Z'
labels: []
dependencies: []
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
An Upgrades tab upgrades the construction yard, barracks, factory, hangar and mini-house buildings for Rules' UpgradeCost; units and towers with UpgradedPrimaryRequired need it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Upgrades tab lists what can be upgraded and upgrades it for its cost over time
- [x] #2 What Rules marks UpgradedPrimaryRequired cannot be made before
- [x] #3 Opponents upgrade too
- [x] #4 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
BuildTab::Upgrades with Entry::Upgrade(role): the yard (600), barracks (800), factory, hangar (1200), refinery pads and allies' buildings, with the disc's pictures (the pad's and the yard's own) and tab symbol. Paid as it goes, cancelled with a right click. Role::needs_upgraded_maker lists Rules' UpgradedPrimaryRequired among what we have; Unit::can_make checks it. Opponents upgrade their yard at once and the rest once they have four fighters and money to spare. Tips say what is missing.
<!-- SECTION:FINAL_SUMMARY:END -->
