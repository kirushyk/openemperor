---
id: TASK-125
title: Flying NIAB of the last mission moves as the original's
status: Done
assignee: []
created_date: '2026-11-21T23:05:00Z'
updated_date: '2026-11-21T23:11:00Z'
labels: []
dependencies: []
ordinal: 125000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Guild's flying NIAB in the final mission circles as an air drone does.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 It moves as the original's does
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The rules mark GUNIAP Circles = TRUE, as the air drone, and give it GUNIAPGun firing GuildRocket_B (300, HeatAir, homing 28/0.3, 7 tiles, reload 30). never_stops now covers it and the other extras marked Circles (GUScavenger, TLINFlyer, StuntFrigate), and it is armed with that rocket. Read as: it should circle as the air drone does.
<!-- SECTION:FINAL_SUMMARY:END -->
