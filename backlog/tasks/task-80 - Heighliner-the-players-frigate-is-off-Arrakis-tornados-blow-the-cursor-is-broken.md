---
id: TASK-80
title: >-
  Heighliner: the player's frigate is off, Arrakis' tornados blow, the cursor is
  broken
status: In Progress
assignee: []
created_date: '2026-11-18T19:01:00Z'
updated_date: '2026-11-18T19:40:00Z'
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
- [x] #1 The player's factory frigate stands where and as the original has it
- [x] #2 No tornados on the Heighliner
- [ ] #3 The cursor works on the Heighliner as on Arrakis
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Tornados now spin up only on sand (Harkonnen Heighliner has none, Atreides 12 cells, Ordos 15). Cursor: picking iterated 4 times from height 0 and flipped between the deck (160) and the void (0) at its edges; it now marches down the ray from the highest ground. Waiting to hear how the frigate is off and whether the cursor is right now.

Box selection: units now counted by where their feet appear on screen, not the spot under them on the sand plane (the deck is 160 up). Bridges: the Heighliner has 91 cells in one-cell passages (NbRock bridges), which the route grid (all four cells open) could not pass; on foot a point now needs two side-by-side open cells, steps must run beside or across open cells, and waypoints sit in the open cells. A sniper walks from its room across the deck.

Frigate: a map object's cell, and a script's point with its offsets, is the first square across and down of the building's squares as it faces. Every frigate on the three Heighliners now stands on open deck (the script's Atreides frigate had 89 of its 144 squares on cliff). A construction yard a script digs in keeps its own anchor.
<!-- SECTION:NOTES:END -->
