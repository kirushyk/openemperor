---
id: TASK-120
title: Heighliner's Mongooses join the player as the script says
status: Done
assignee: []
created_date: '2026-11-21T21:38:00Z'
updated_date: '2026-11-21T21:42:00Z'
labels: []
dependencies: []
ordinal: 120000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approaching the neutral Atreides Mongooses on the Heighliner, the Mentat says they are under the player's command, but they stay neutral.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The units the script hands over become the player's
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The Atreides Heighliner script says line 536 and calls F91(v18, F14()) once the player's units see side v18 (its Mongooses and general). F91(a, b) is now Battle::hand_over: every unit of side a takes side b's owner and colour. Also dropped the draw_flat left unused by the hourglass change.
<!-- SECTION:FINAL_SUMMARY:END -->
