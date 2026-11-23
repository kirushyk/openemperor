---
id: TASK-175
title: Spice mounds, debris, scrap and splats
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 175000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Kinds we have nothing for: [SpiceMoundTypes] (rules 0x20f28c; SpiceMound.CPP 0x22112c, used from 0x141db6; spice.ini 0x221170; 'SpiceMound on map with no spice' 0x2263f0), the spice blooms of the original; [DebrisTypes] (0x20ec4c; debris.cpp 0x1f4bf8 at 0xa103c), [ScrapTypes] (0x20f1c4) and [SplatTypes] (0x20f354; Splat.CPP 0x22117c at 0x143417). Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Spice mounds grow and burst into spice as the original's
- [ ] #2 Destroyed things throw their rules' debris and leave scrap and splats
<!-- AC:END -->
