---
id: TASK-83
title: The original tutorial, fully playable
status: In Progress
assignee: []
created_date: '2026-11-18T23:44:00Z'
updated_date: '2026-11-20T20:43:00Z'
labels: []
dependencies: []
ordinal: 83000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User: 'make tutorial and final mission fully playable'. ATTutorial on #x1 Player Tutorial; its script uses F24, F29, F30, F36, F38, F42, F63, F64, F71, F82, F84, F85, F96, F97, F107, F108, K135, K144 that we do not know yet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The tutorial is fought on #x1 by ATTutorial, its Mentat's lines and steps as on the disc
- [ ] #2 Every function and keyword it uses does what the original's does
- [x] #3 It can be played through to its end
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-20: the Tutorial button runs ATTutorial on #x1 (our own tutorial when the disc lacks them); its enemy does only what the script says. F24, F29, F30, F36, F38, F42, F64, F71, F84, K135 and K144 done; comparisons re-read as K169 !=, K171 <=, K173 <; speech numbers fixed (task 86); a yard F21 brings stands dug in; a refinery a script puts down brings no harvester. Played headless to Victory in about 15 minutes with a harness standing in for the player. Still open: F82 (the worm; there are no worms yet, task 87), and F63, F85, F96, F97, F107, F108, whose effects are unknown and which the tutorial runs without.
<!-- SECTION:NOTES:END -->
