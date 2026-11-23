---
id: TASK-162
title: Key bindings and the commands we lack
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 162000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's commands (names at file 0x1f9978-0x1f9e18) include ones we don't have: Pause, HarvRetreat, PickUp, BuildingLeft/Right, DefBookmark1-10 and GoToBookmark1-10, ViewTeam1-9, FullRetreat, BaseRetreat, UnitTransfer, Formation, Allegiance, the tab keys, DeleteWaypoint, InsertWaypoint, CycleElite, SelectAll, Prev/NextUnit, Follow, Taunt, Scatter, RadarAlert, ZoomIn/Out. Bindings are kept in KEYS.MAP (0x1f9f28, read at 0xd9003; 'using defaults' 0x1f9f74) and changed on the KeyDefiner screen. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each command the original has, that the battle can use, has a key
- [ ] #2 Bookmarks, scatter, follow and next/previous unit work
- [ ] #3 Keys can be changed and are kept between runs
<!-- AC:END -->
