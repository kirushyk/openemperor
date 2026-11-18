---
id: TASK-53
title: Turn scripted and map buildings the right way round
status: Done
assignee: []
created_date: '2026-11-18T14:39:00Z'
updated_date: '2026-11-18T14:43:00Z'
labels: []
dependencies: []
ordinal: 53000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Since scripted facings were taken, buildings stand rotated 180 degrees.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Scripted and map buildings face 180 degrees from the last round's facing, on the same squares
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Facing::numbered now starts from RightTop; put anchors the back row on the point so plots are unchanged (tests (40,38,7,5) and (87,7,18,6) hold).
<!-- SECTION:FINAL_SUMMARY:END -->
