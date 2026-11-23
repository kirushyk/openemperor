---
id: TASK-87
title: Sandworms, as the tutorial's script calls one up
status: To Do
assignee: []
created_date: '2026-11-20T20:43:00Z'
updated_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 87000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's worms: they come up under what stands on sand and swallow it. The tutorial's script calls one up with F82 at the wave of Harkonnen it sent, and the Mentat warns of it; none comes, as the game has no worms yet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 F82(place) brings a worm up at that place
- [ ] #2 A worm swallows units on sand near it, not on rock, and goes down again
- [ ] #3 It is seen and heard as the original's
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
GAME.EXE (file offsets; +0x400000 for the address; map in docs/game-exe.md): see TASK-156 for the worms' rules and code: AiSurfaceWorm.cpp (0x1eade0), CAiWormManager (0x1eda88, at 0x6d9b2), unitWorm.cpp (0x222bc0, from 0x171578); script functions ForceWormStrike F82 (case 0xf2c02, 0x1f) and SideAttractsWorms F80 (case 0xf2a99, 0xa9).
<!-- SECTION:NOTES:END -->
