---
id: TASK-63
title: Sand bikes cannot squash infantry
status: Done
assignee: []
created_date: '2026-11-18T16:00:00Z'
updated_date: '2026-11-18T16:04:00Z'
labels: []
dependencies: []
ordinal: 63000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A sand bike runs over soldiers and kills them; in the original it cannot.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A sand bike never crushes infantry
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Role::crushes from the rules' Crushes = TRUE; can_run_over and can_squash require it. Test with a sand bike.
<!-- SECTION:FINAL_SUMMARY:END -->
