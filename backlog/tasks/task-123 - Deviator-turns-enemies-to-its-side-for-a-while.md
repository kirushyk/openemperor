---
id: TASK-123
title: Deviator turns enemies to its side for a while
status: Done
assignee: []
created_date: '2026-11-21T23:04:00Z'
updated_date: '2026-11-21T23:08:00Z'
labels: []
dependencies: []
ordinal: 123000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Ordos deviator's gas makes the enemy unit it hits its owner's for the rules' DeviateDuration, then it returns.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A deviator fires its Deviate_B at enemy machines and infantry within its MaxRange
- [x] #2 What it hits is its owner's for DeviateDuration (400 updates), then goes back
- [x] #3 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
WeaponKind::Deviate (Deviate_B: 250 damage, Deviate_W shares, 150-update reload, speed 16, straight, 8 tiles) is the deviator extra's weapon. hit() deviates an enemy vehicle or soldier it hits: Unit::deviate keeps who it was, takes the shooter's owner and colour for 16 s (DeviateDuration 400), and come_back gives it back; hit again, the time restarts. Its look in flight is the usual missile's (task 124).
<!-- SECTION:FINAL_SUMMARY:END -->
