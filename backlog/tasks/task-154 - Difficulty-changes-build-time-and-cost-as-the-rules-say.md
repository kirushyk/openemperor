---
id: TASK-154
title: Difficulty changes build time and cost as the rules say
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 154000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The rules give each kind EasyBuildTime/NormalBuildTime/HardBuildTime (file 0x21b9c4, 0x21ba8c, 0x21bb54) and EasyBuildCost/NormalBuildCost/HardBuildCost (0x21bc1c, 0x21bce4, 0x21bdac), percentages for the computer's sides. The AI's own difficulty is ai_difficulty.ini (file 0x1e63a8, read at 0x31aaf; 'expecting level from 1 to 9', 0x1e6338). Our skirmish difficulty radios set nothing of these. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The computer's sides build at the time and cost the chosen difficulty's rules give
- [ ] #2 A test checks a kind's time and cost at each difficulty
<!-- AC:END -->
