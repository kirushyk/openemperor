---
id: TASK-46
title: Play the original's tutorial
status: Done
assignee: []
created_date: '2026-11-18T11:25:00Z'
updated_date: '2026-11-21T05:24:00Z'
labels: []
dependencies: []
ordinal: 46000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Tutorial button plays our stand-in: skirmish map 0 with no house. The disc has the original's: map '#X1 Player Tutorial' run by ATTutorial.tok in MISSIONS0001 (942 lines). It calls functions the interpreter lacks: F12, F24, F29, F30, F32, F36, F37, F38, F42, F44, F54, F59, F60, F63, F64, F71, F82, F84, F85, F96, F97, F101, F107, F108, F112, F113, and keywords K131-K135, K144, K170, K173. Their names are not on the disc, so each has to be worked out from its use. CAMPAIGN0001 also has 'tutorial territories.txt' and 'tutorial connections.txt'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The Tutorial button fights on #X1 under ATTutorial.tok
- [ ] #2 Every function and keyword the script uses does what its use shows, or is documented as unknown in docs/tok-format.md
- [ ] #3 The tutorial can be played from start to its end
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Duplicate of task 83, under which the original tutorial now runs on #x1 by ATTutorial.
<!-- SECTION:FINAL_SUMMARY:END -->
