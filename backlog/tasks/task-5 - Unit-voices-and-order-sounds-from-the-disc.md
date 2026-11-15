---
id: TASK-5
title: Unit voices and order sounds from the disc
status: Done
assignee: []
created_date: '2026-11-14T01:28:00Z'
updated_date: '2026-11-14T01:28:00Z'
labels: []
dependencies: []
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Play the original's sounds from SFX/AUDIO.BAG when the player selects units and gives orders.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Selecting one unit, or a group of one kind, plays one of its own lines as the Sfx tables name them
- [x] #2 A mixed group plays its house's group line
- [x] #3 Sending units somewhere plays their move line, and an attack order their attack line
- [x] #4 Lines play on a channel of their own over the music, each cutting off the one before
- [x] #5 Without the disc nothing plays
- [x] #6 Tests cover reading the tables, decoding the bag's sounds and choosing the line
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
src/sounds.rs reads the bag and tables, decodes IMA ADPCM in 512-byte blocks; game.rs records Said replies; the shim has a second channel (oe_play_line).
<!-- SECTION:FINAL_SUMMARY:END -->
