---
id: TASK-160
title: 'The original''s AI: patterns and units'
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 160000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Patterns the AI triggers (strings 0x1e7988-0x1e8024): retreat when losing (AiPatternRetreat.cpp; 0x1e7c30 used at 0x3f3f1), sell, save money, replace harvesters, cancel tactics, order from the starport (0x1e7f44 used at 0x405d4), fire its superweapon (0x1e7eb8 used at 0x40288), captured buildings, and go for the kill. Unit handlers AiUnit*.cpp (paths 0x1ed028-0x1ed690): air drones, carryalls lifting units, crushers, the deviator, EITS, engineers, the Ix infiltrator, ornithopters going to rearm, saboteurs, scouts. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The computer retreats, sells, saves, uses the starport and fires its superweapon as the original's patterns say
- [ ] #2 Its special units act as their AiUnit handlers do
<!-- AC:END -->
