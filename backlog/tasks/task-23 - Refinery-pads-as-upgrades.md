---
id: TASK-23
title: Refinery pads as upgrades
status: Done
assignee: []
created_date: '2026-11-16T15:20:00Z'
updated_date: '2026-11-16T15:37:00Z'
labels: []
dependencies: []
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A refinery starts with one pad and one harvester; each RefineryDock upgrade (1200) adds a pad and a harvester, up to three.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A new refinery has one working pad and brings one harvester
- [x] #2 Each of two upgrades adds a pad and a harvester for Rules' cost and time
- [x] #3 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Unit::upgrades counts a refinery's further pads; working_pads gives them in Rules' DeployTile order (middle, then each side). Each upgrade costs 1200 over 18 s, uses 20 power and brings a harvester by carryall. Drawn with all three pads still (logged).
<!-- SECTION:FINAL_SUMMARY:END -->
