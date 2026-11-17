---
id: TASK-32
title: Find the game's other discs and use their music
status: Done
assignee: []
created_date: '2026-11-17T07:30:00Z'
updated_date: '2026-11-17T07:30:00Z'
labels: []
dependencies: []
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Look for EMPEROR1 to 4 and play battle music from every music bag found.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Whichever discs are mounted are found, and the install disc among them
- [x] #2 Battle music cycles through every bag's long tracks
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
src/discs.rs finds /Volumes/EMPEROR1..4, tells the install disc, and lists every music bag (the first disc's INSTALL/DATA/MUSIC/music.bag, a house disc's MUSIC.BAG). main uses the install disc it finds in place of the fixed path; music::battle_tracks takes all the bags and goes round their long tracks. Inspected disc 2: 20 Atreides movies in MOVIES0001.RFD, 13 music tracks, 1163 dialogue lines.
<!-- SECTION:FINAL_SUMMARY:END -->
