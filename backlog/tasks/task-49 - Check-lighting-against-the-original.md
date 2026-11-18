---
id: TASK-49
title: Check lighting against the original
status: Done
assignee: []
created_date: '2026-11-18T11:58:00Z'
updated_date: '2026-11-18T12:51:00Z'
labels: []
dependencies: []
ordinal: 49000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user recalls the original's sun as weaker than the Mongoose searchlights; ours looks the other way round. Compare the maps' test.lit and our sun, ambient and searchlight strengths
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Sun, ambient and searchlight strengths follow what the disc says, or the gap is documented
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
4b97e9e: models lit by test.lit's second, darker pair; searchlights at full white; burning lights added on top of the ground's wash, which used to halve them. The original's searchlight numbers are in GAME.EXE code; documented as a guess.
<!-- SECTION:FINAL_SUMMARY:END -->
