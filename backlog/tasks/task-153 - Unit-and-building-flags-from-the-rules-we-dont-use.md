---
id: TASK-153
title: Unit and building flags from the rules we don't use
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 153000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Flags and figures of [UnitTypes] and [BuildingTypes] in GAME.EXE's rules table (file 0x20e79c, 0xc8-byte records) that we don't read: CanFly 0x212874, CanReverse 0x212acc, CanMoveAnyDirection 0x212b94, FaceCamera 0x212c5c, Crushable 0x217b44, CanBeDeviated 0x216f8c, CantBeLeeched 0x212a04, CanBeEngineered 0x217cd4, CanBePrimary 0x217d9c, CanBeRepaired 0x21d38c, CanDie 0x216ec4, Projectable 0x216dfc, HideUnitOnRadar 0x21d2c4, NumInfantryWhenGone 0x21d134, ExcludeFromCampaignLose 0x21d51c, GiftsToKiller 0x21e7dc, CrateGift 0x2120a4, UnstealthRange 0x212234, Stealthed 0x211f14, CanSelfRepairShield 0x211e4c, SpiceCapacity 0x216ba4, UnloadRate 0x216c6c, Dockable 0x216adc, GetUnitWhenBuilt 0x216a14, Resource 0x21694c, DisableIfNoSpiceOnMap 0x216884, HoldBack 0x2167bc, RemoveSpice 0x217a7c, SpecialGround 0x2175cc, SinkAmount/SinkSpeed 0x217824/0x2178ec, Mech/MechSpeed 0x217694/0x21775c, HitScore 0x21662c, ExtraFireRate 0x21d06c, SecondaryBuilding/UpgradedSecondaryRequired 0x2154fc/0x215b3c, SoundFile 0x215ccc. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each listed flag or figure is used as its name says, or docs/not-implemented.md says why not
- [ ] #2 Destroyed buildings leave NumInfantryWhenGone soldiers
- [ ] #3 A harvester holds SpiceCapacity and unloads at UnloadRate
<!-- AC:END -->
