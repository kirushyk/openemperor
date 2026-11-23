---
id: TASK-172
title: Campaign reserves and the other houses' turns from Difficulty.txt
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 172000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Difficulty.txt (file 0x1f1b18) with [Random], [Personality], [Phase], [RRDifference] (0x1f1b28-0x1f1b4c) gives reserves and reinforcements by phase; ai-only battles are worked out by CalculateAiOnlyGameResults (0x1f2410-0x1f2480). Reserves are moved on the map (MoveReserves 0x1f1a14, used at 0x8d95b) and retreated units go back to them (0x1f22ec, at 0x92f9c). The rules' UnitValueAttacker/Defender/Reserves/InitialReinforcements/SubsequentReinforcements, TicksBetweenReinforcements(Variation), TicksBeforeReinforcementsForMessage (0x21c324-0x21c89c), ReinforcementValue (0x21be74), CampaignAttackMoney/DefendMoney (0x21e64c, 0x21e714). Ours are by chance. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The other houses attack and win between themselves as Difficulty.txt and the reserves say
- [ ] #2 Reserves can be moved and come into battles as reinforcements by the rules' figures
<!-- AC:END -->
