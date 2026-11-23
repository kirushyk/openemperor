---
id: TASK-177
title: LAN multiplayer
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 177000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
GAME.EXE plays over a LAN (CLANWrapper, 0x1fc6d4-0x1fc88c; UDP transport 0x1fbff0) and Westwood Online (WOL..., chat events 0x1f3900-0x1f4174, servers gone). Game set-up changes are messages (CHANGE_MAP, CHANGE_MONEY, CHANGE_CRATES... 0x1efce8-0x1effe4), the game keeps in step by orders (CNetworkAdmin, 0x20bafb-0x20c564) and can record them (game.rec 0x20e0d4, at 0x11e636). The co-op campaign is WolCoopSetupMgr (0x1efca8). Low priority. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Two copies on one network can play a skirmish together
<!-- AC:END -->
