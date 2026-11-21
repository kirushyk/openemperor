---
id: TASK-116
title: Hourglass cursor as the original's
status: Done
assignee: []
created_date: '2026-11-21T20:57:00Z'
updated_date: '2026-11-21T21:13:00Z'
labels: []
dependencies: []
ordinal: 116000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Ours is drawn filled and still; the original's is an empty outline that moves.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The hourglass is drawn as the original's: outlined, and moving
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Light pictures of the disc's interface scenes are now scaled by their alpha as they are cut into tiles, so the hourglass's (and pointer's) triangle picture adds nothing where it is see-through: it was adding white there, filling the hourglass. The hourglass is drawn tipped 45 degrees (our choice) so its egg's turning over end to end, invisible from straight above, shows; the test checks its bigger end swaps sides.
<!-- SECTION:FINAL_SUMMARY:END -->
