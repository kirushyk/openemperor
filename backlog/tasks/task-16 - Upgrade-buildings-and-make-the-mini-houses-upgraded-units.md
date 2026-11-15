---
id: TASK-16
title: Upgrade buildings, and make the mini-houses' upgraded units
status: To Do
assignee: []
created_date: '2026-11-14T05:07:00Z'
labels: []
dependencies: []
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rules.txt gives buildings an UpgradeCost and UpgradeTechLevel. The mini-houses' Projector, Leech, ADV Sardaukar, ADV Fremen and NIAB Tank need an upgraded main building (UpgradedPrimaryRequired). Our game has no upgrades yet. The IMDropShip is called in, not built.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A building can be upgraded for its UpgradeCost, as in the original
- [ ] #2 IXProjector, TLLeech, IMADVSardaukar, FRADVFremen and GUNIABTank can be made once their mini-house's main building is upgraded, and not before
- [ ] #3 Tests cover upgrading and the units it allows
<!-- AC:END -->
