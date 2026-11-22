---
id: TASK-130
title: Light triangles left after vehicles blow up
status: Done
assignee: []
created_date: '2026-11-22T01:10:00Z'
updated_date: '2026-11-22T01:33:00Z'
labels: []
dependencies: []
ordinal: 130000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
After some vehicles explode, glowing triangles stay where they were.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Nothing of a destroyed vehicle's lights or flashes is left glowing
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
A vehicle blown apart drew its disc model's lamp glows as decals, which break_apart did not move, so they stayed where it stood; its glows are now left out once it is destroyed.
<!-- SECTION:FINAL_SUMMARY:END -->
