---
id: TASK-93
title: No searchlight pool around an MCV
status: Done
assignee: []
created_date: '2026-11-21T05:45:00Z'
updated_date: '2026-11-21T05:46:00Z'
labels: []
dependencies: []
ordinal: 93000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
An MCV throws a sweeping pool of light on the ground though it has no searchlight beam with the disc's model; it is not wanted.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 An MCV casts no searchlight pool on the ground
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The pool from the MCV's own fallback searchlight was added even when the disc's model, which has no beam, drew it. It is gone; its headlights stay as every vehicle's do. Tested.
<!-- SECTION:FINAL_SUMMARY:END -->
