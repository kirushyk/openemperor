---
id: TASK-132
title: Every unit plays its model's animations
status: Done
assignee: []
created_date: '2026-11-22T01:10:00Z'
updated_date: '2026-11-22T01:45:00Z'
labels: []
dependencies: []
ordinal: 132000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The buzzsaw's disc does not spin; check every unit's model for animations it has and we do not play.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The buzzsaw's saw spins as it does in the original
- [x] #2 Every unit's model's sequences are played where the original plays them, or listed as not
- [x] #3 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The buzzsaw's wheel froze because firing played its whole model's Fire 0; firing on the move now plays fire only on the parts Fire 0 moves and Move does not. Also played now: Turn Left/Right, Idle 0/1, Crouch/CrouchFire, Land/Takeoff, Harv Unload Start/Hold/End. The runs still not played are listed in not-implemented.
<!-- SECTION:FINAL_SUMMARY:END -->
