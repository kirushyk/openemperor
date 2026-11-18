---
id: TASK-55
title: Make the tornado see-through
status: Done
assignee: []
created_date: '2026-11-18T14:39:00Z'
updated_date: '2026-11-18T14:49:00Z'
labels: []
dependencies: []
ordinal: 55000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The tornado looks too opaque.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The tornado is drawn as see-through as the original's model and art say
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
See-through veils keep the model's normals and are culled when the camera sees their back (Model::sees_front); glows stay two-sided. Frame checked: windtraps show through the funnel.
<!-- SECTION:FINAL_SUMMARY:END -->
