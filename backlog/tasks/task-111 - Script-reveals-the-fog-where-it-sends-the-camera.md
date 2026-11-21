---
id: TASK-111
title: Script reveals the fog where it sends the camera
status: Done
assignee: []
created_date: '2026-11-21T19:58:00Z'
updated_date: '2026-11-21T20:01:00Z'
labels: []
dependencies: []
ordinal: 111000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The tutorial calls F97(place, 20) as it shows a beacon and the enemy base: it lifts the fog there.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 F97(place, n) uncovers the fog n squares round the place for the player
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
F97(place, n) calls Battle::uncover, which marks the fog's ever-seen distances n squares round the place: known (dimmed), not seen. Tests: the script's call, and the fog uncovered and unseen.
<!-- SECTION:FINAL_SUMMARY:END -->
