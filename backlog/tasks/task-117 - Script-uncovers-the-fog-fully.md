---
id: TASK-117
title: Script uncovers the fog fully
status: Done
assignee: []
created_date: '2026-11-21T20:57:00Z'
updated_date: '2026-11-21T20:58:00Z'
labels: []
dependencies: []
ordinal: 117000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
F97 makes the land only known (grey); the original opens it fully, as if seen.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Land F97 uncovers shows as seen, not dimmed
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fog keeps an opened layer F97 spreads into; each update starts what is seen now from it, so the land stays seen, not dimmed.
<!-- SECTION:FINAL_SUMMARY:END -->
