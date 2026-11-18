---
id: TASK-66
title: Tanks squash nearby infantry rather than shoot them
status: Done
assignee: []
created_date: '2026-11-18T16:05:00Z'
updated_date: '2026-11-18T16:09:00Z'
labels: []
dependencies: []
ordinal: 66000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user: it is easier for a tank to squash a few nearby soldiers than fire at them many times. Units' own choices should prefer running over infantry close by when they crush.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A crushing vehicle with enemy soldiers close by drives over them instead of standing and firing
- [x] #2 One that does not crush, or whose targets are far, keeps firing
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
units::run_down_the_near (idle crushers, RUN_DOWN_REACH 4 tiles) and answer_attacker preferring RunOver for a near attacker. Tests for both and for non-crushers and far targets.
<!-- SECTION:FINAL_SUMMARY:END -->
