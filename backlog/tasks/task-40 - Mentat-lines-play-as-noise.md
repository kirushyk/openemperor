---
id: TASK-40
title: Mentat lines play as noise
status: Done
assignee: []
created_date: '2026-11-18T11:25:00Z'
updated_date: '2026-11-18T11:25:00Z'
labels: []
dependencies: []
ordinal: 40000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The house discs keep 639 of their lines as MP3 (bag format 0x20), which were read as raw 16-bit sound
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Lines kept as MP3 in a bag decode through FFmpeg
- [x] #2 A disc test decodes the Atreides welcome to more than two seconds
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
91de8b5: MP3 lines decode through FFmpeg.
<!-- SECTION:FINAL_SUMMARY:END -->
