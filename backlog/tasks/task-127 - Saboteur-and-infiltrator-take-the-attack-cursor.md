---
id: TASK-127
title: Saboteur and infiltrator take the attack cursor
status: Done
assignee: []
created_date: '2026-11-21T23:09:00Z'
updated_date: '2026-11-21T23:28:00Z'
labels: []
dependencies: []
ordinal: 127000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
With a saboteur or an infiltrator selected, the player should be able to point it at a target with the attack cursor, not only blow it up by deploying.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Over an enemy building or unit, a selected saboteur or infiltrator shows the attack cursor
- [x] #2 Clicked, it goes to the target and blows up there
- [x] #3 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Unit::can_bomb (saboteur, Ixian infiltrator, not deployed; an enemy on the ground it can see) is part of can_tend, so the attack order and the cursor's choice take it, and the cursor shows Attack for it. chase_targets walks it up to the target and, within its radius, the target's and BOMB_MARGIN (16), digs it in, which sets off its bomb as deploying does.
<!-- SECTION:FINAL_SUMMARY:END -->
