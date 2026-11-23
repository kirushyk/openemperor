---
id: TASK-142
title: 'Script events: things delivered, built and attacked'
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 142000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The scripts wait on events we never raise: EventObjectDelivered F39 (case file 0xf2eda, 0x92 bytes), EventObjectConstructed F40 (27 uses in 18 scripts; 0xf2f6c, 0xb5), EventObjectTypeConstructed F41 (267/30; 0xf3021, 0xbe) and EventObjectAttacksSide F43 (0xf314a, 0x47). EventSideAttacksSide F42 and EventObjectDestroyed F44 are done. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The four events become true when the original's would, and only once each as the original's case code has it
- [ ] #2 Tests cover each event
<!-- AC:END -->
