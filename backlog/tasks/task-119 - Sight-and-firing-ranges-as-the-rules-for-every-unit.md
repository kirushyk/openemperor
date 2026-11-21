---
id: TASK-119
title: Sight and firing ranges as the rules' for every unit
status: Done
assignee: []
created_date: '2026-11-21T21:34:00Z'
updated_date: '2026-11-21T21:38:00Z'
labels: []
dependencies: []
ordinal: 119000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Check each original unit's ViewRange and its weapons' Range against Rules.txt and make ours the same.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every unit's sight is its rules' ViewRange
- [x] #2 Every weapon reaches its rules' Range
- [x] #3 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Compared every unit's ViewRange and bullet MaxRange (through TurretAttach/TurretNextJoint) with ours. Fixed: deployed yard sees 10 (ConYard) not the MCV's 12; buzzsaw 7, Sardaukar 9, ATGeneral 9, contaminator 1, Fedaykin 5, Maker 9, SM quad 10 tiles. The rest matched. Logged: InfRock second figures and range bonus, and our sight at least our reach.
<!-- SECTION:FINAL_SUMMARY:END -->
