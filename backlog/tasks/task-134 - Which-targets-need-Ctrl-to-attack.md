---
id: TASK-134
title: Which targets need Ctrl to attack
status: To Do
assignee: []
created_date: '2026-11-22T01:14:00Z'
updated_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 134000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As remembered, hanged figures (gallows) could not be attacked without Ctrl. Check whether the rules have such a flag and where else it applies, and follow it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The rules' flag for what is attacked only by force is found and named
- [ ] #2 Everything it marks is attacked only with Ctrl
- [ ] #3 Tests cover it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
No rules key found: the gallows' sections have House Incidental, CountsForStats FALSE, no ExplosionType; the factory frigates CanBeEngineered FALSE, Factory TRUE. Their models have SLCT boxes. Ours: every neutral thing is attacked by a plain click. Asked the user which things need Ctrl in the original.

GAME.EXE (file offsets; +0x400000 for the address; map in docs/game-exe.md): the cursor names table (0x1f4898-0x1f4b64: CATTACK, CMOVE, ATTACK_RADAR...) and the cursors' models cursors\CU_%s_H0.XBF (0x1f4b6c).
<!-- SECTION:NOTES:END -->
