---
id: TASK-170
title: 'Spoken messages: priority, interrupting, and the events we never raise'
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 170000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Dialog.txt's Priority, Interrupt and TimeOut (messages 0x1f4eb8-0x1f5020) decide which line plays. The events (names 0x1f5214-0x1f5498) we have no trigger for: GameSaved, GameLoaded, ReinforceArr, ReinforceApp, RetreatFailed, RetreatSuccess, RetreatStartEnemy, WayMoved, WayDeleted, TargDest, CarryAttack, Wormstrike, ContAttack, LeechAttack, BldgCaptured, BldgStolen, IncomingDHand, IncomingChaos, IncomingHawk, SpecWepStart, NewConOpt, CannotBuild. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A line's priority and interrupt decide whether it cuts in, waits or is dropped
- [ ] #2 Each event above is said when it happens in the battle
<!-- AC:END -->
