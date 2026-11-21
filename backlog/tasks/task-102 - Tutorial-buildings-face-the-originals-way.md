---
id: TASK-102
title: Tutorial buildings face the original's way
status: Done
assignee: []
created_date: '2026-11-21T18:56:00Z'
updated_date: '2026-11-21T19:11:00Z'
labels: []
dependencies: []
ordinal: 102000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The tutorial puts its buildings with facings 0 and 2 and they stand 180 degrees round; the first mission's 1 and 3 were already right. The numbers run the other way round.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Facings 0 and 2 turn 180 degrees, 1 and 3 stay
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Facing::numbered now goes LeftBottom, RightBottom, RightTop, LeftTop: out along +z, +x, -z, -x. 1 and 3 are as before, 0 and 2 turn 180 degrees, on the same squares (a script's yard keeps its back row on its point). This also turns the first mission's and the maps' things given 0 or 2.
<!-- SECTION:FINAL_SUMMARY:END -->
