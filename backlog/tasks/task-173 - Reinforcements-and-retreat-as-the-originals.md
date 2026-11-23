---
id: TASK-173
title: Reinforcements and retreat as the original's
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 173000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
CReinforcements (file 0x20e264, used at 0x123374) brings a battle's reinforcements in with ReinforceArr/ReinforceApp(New) (0x20e20c, 0x20e21c) and a timer on the sidebar (ReinforcementTimer 0x1f6964, at 0xb7d5b). CRetreat (0x20e6ac, at 0x124973) takes RetreatNumUnits (0x20e658) off by a beacon flare (BeaconFlare 0x20e6a0, at 0x124772), asked in RetreatPopUp (0x1f6b30, at 0xb9ab9). Ours asks in the chat and gives up at once. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Retreat asks in the original's pop-up and takes units off by the beacon
- [ ] #2 Reinforcements arrive with their messages and the sidebar's timer
<!-- AC:END -->
