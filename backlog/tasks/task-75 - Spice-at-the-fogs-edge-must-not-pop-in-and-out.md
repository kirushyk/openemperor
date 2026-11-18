---
id: TASK-75
title: Spice at the fog's edge must not pop in and out
status: Done
assignee: []
created_date: '2026-11-18T18:28:00Z'
updated_date: '2026-11-18T18:33:00Z'
labels: []
dependencies: []
ordinal: 75000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User: 'also very visible how spice on edge of fog appears disappears'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Spice fades with the fog's smooth edge instead of appearing and disappearing per cell
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Spice patches' corners now carry fog::edges_at like the ground's, so the shader fades them along the round edge; seen_at removed.
<!-- SECTION:FINAL_SUMMARY:END -->
