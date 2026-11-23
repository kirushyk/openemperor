---
id: TASK-150
title: General rules we don't use
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 150000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
GAME.EXE reads Rules.txt with a keyword table at file 0x20e79c-0x21ebc4 (333 records of 0xc8 bytes; loader CWorldAttrib::LoadRulesTextFile, its message at file 0x21f2cc, used at 0x12bb2e). Of its [General] keys these are not used: SpiceValue (0x20f804), FogRegrowRate (0x20f8cc), RearmRate (0x20f994), RepairCostPercent (0x20fb24), SellCostPercent (0x20fbec), CashDeliveryWhenNoSpiceAmountMin/Max and FrequencyMin/Max (0x210164-0x2103bc), AdvCarryallPickupEnemyDelay (0x210484), RepairTileRange (0x2109fc), MaxBuildingPlacementTileDist (0x210b8c), MinCarryTileDist (0x210c54), SuppressionDelay/SuppressionProb (0x210d1c, 0x210de4) with CanBeSuppressed, InfDamageRangeBonus (0x210f74), HeightRangeBonus and HeightBonusLevel1/2 (0x21103c-0x2111cc) with GetsHeightAdvantage, BulletGravity (0x211294), HarvReplacementDelay (0x21e4bc). Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each listed key's figure is used for what its name says, or docs/not-implemented.md says why not
- [ ] #2 Selling and repairing cost the rules' percentages
- [ ] #3 Spice pays the rules' SpiceValue a load, and with no spice left cash is delivered as the rules say
<!-- AC:END -->
