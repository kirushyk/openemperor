---
id: TASK-146
title: Script orders to the computer's sides
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 146000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The scripts drive their sides through the AI: SideAIControl F63 (158 uses; case file 0xf22a9), SideAIAggressiveTowards F65 (125; 0xf235a), SideAIBehaviourAggressive/Retreat/Normal F66-F68 (0xf246a, 0xf24bc, 0xf250e), SideAIEncounterIgnore/Attack F69/F70 (0xf2560, 0xf25e4), SideAIStop F72 (0xf274f), SideAIAttackObject F73 (0xf281c), SideAIGuardObject F74 (0xf289f), SideAIExitMap F75 (141 uses; 0xf29b4), SideAIEnterBuilding F76 (0xf2a16), SideAIBehaviourDefensive F77 (0xf3f50), SideAIHeadlessChicken F78 (0xf3fba), SideAIShuffle F79 (0xf4022) and SetThreatLevel F155 (0xf42e0). The AI's side of these is CAiSide's mission commands: messages at file 0x1e4194-0x1e43e8 and 0x1ea74c-0x1ea98c ('Side %d Mission command ...'), and the tactics they start ('Move Off Map Team %d', 'Guard Object T', 'Headless Chicken T', 'Shuffle Tactic'). Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each listed order makes the side's units do what the original's mission command does
- [ ] #2 Units sent off the map leave it and are gone
- [ ] #3 Tests cover the orders
<!-- AC:END -->
