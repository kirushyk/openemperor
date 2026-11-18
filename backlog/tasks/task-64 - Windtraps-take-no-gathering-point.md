---
id: TASK-64
title: Windtraps take no gathering point
status: Done
assignee: []
created_date: '2026-11-18T16:00:00Z'
updated_date: '2026-11-18T16:05:00Z'
labels: []
dependencies: []
ordinal: 64000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A windtrap makes no units, so it should take no rally point.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Selecting a windtrap and clicking on the map sets no gathering point for it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Role::takes_gathering_point (makers and the starport) used by set_gathering and would_go_here; test with a windtrap.
<!-- SECTION:FINAL_SUMMARY:END -->
