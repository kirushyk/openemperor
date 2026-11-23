---
id: TASK-164
title: The score screen after a battle
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 164000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Each side's results are counted (strings 0x1f05c8-0x1f072c: units and buildings built, lost, killed, left, captured, infantry and aircraft, crates found, spice harvested, credits left) and shown on the score screen (ScoreSkirmish/ScoreCampaign; 'Starting up score screen' 0x21f410, used at 0x12cb7b; ScoreMoneyData, ScoreEfficiencyData 0x21f4e0 at 0x12d434; units01/buildings01 bars). Kinds count by CountsForStats (0x21e8a4). We go back to the menu. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each side's results are counted as the original's
- [ ] #2 The score screen shows them after a skirmish and after a campaign battle
<!-- AC:END -->
