---
id: TASK-167
title: The radar as the original's script lays it out
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 167000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Radar\%s.ini (file 0x20de44, read at 0x119e38) gives the radar its SweepMesh, AlertMesh and LocMesh (0x20df10-0x20df7c) and its shroud: FogAmount, FogAlpha, AdditiveBlend, Layers, LayerColour (0x20e018-0x20e050). SoundRadarOn/Off (0x21b384, 0x21b44c), the outpost's radar and HideUnitOnRadar (0x21d2c4) go with it. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The minimap sweeps, alerts and marks the view as the radar script says
- [ ] #2 Units the rules hide do not show on it
<!-- AC:END -->
