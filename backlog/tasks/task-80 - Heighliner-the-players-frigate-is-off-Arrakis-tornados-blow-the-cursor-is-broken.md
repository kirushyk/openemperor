---
id: TASK-80
title: >-
  Heighliner: the player's frigate is off, Arrakis' tornados blow, the cursor is
  broken
status: In Progress
assignee: []
created_date: '2026-11-18T19:01:00Z'
updated_date: '2026-11-18T19:07:00Z'
labels: []
dependencies: []
ordinal: 80000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User, playing the Atreides Heighliner: 'our atr frigate at heighlighner is a bit off, there are tornados from arrakis, mouse cursor is completely broken'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The player's factory frigate stands where and as the original has it
- [x] #2 No tornados on the Heighliner
- [ ] #3 The cursor works on the Heighliner as on Arrakis
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Tornados now spin up only on sand (Harkonnen Heighliner has none, Atreides 12 cells, Ordos 15). Cursor: picking iterated 4 times from height 0 and flipped between the deck (160) and the void (0) at its edges; it now marches down the ray from the highest ground. Waiting to hear how the frigate is off and whether the cursor is right now.
<!-- SECTION:NOTES:END -->
