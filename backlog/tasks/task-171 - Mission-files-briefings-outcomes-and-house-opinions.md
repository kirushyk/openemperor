---
id: TASK-171
title: 'Mission files: briefings, outcomes and house opinions'
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 171000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Each campaign mission is a text file (AT*.txt, OR*.txt, HK*.txt at file 0x202d30-0x202d40, listed at 0xf7ab6) with a [HEADER] (0x1fdc30-0x1fe068: SubHouse, Lifespan, Phase, PlayerHouse, AIHouse, Incidental, Chain, MiniBriefing, Briefing, DefendMission, AttackMission, Difficulty, AllySA/FR/IX/TL/GU) and [success]/[fail] blocks with Debrief, Friendly, Unfriendly, IncreaseFriendly, DecreaseFriendly. Houses' opinions of each other change by them (HOUSE OPINIONS 0x202b58, used at 0xf7353; 'opinon ... has improved' 0x202adc at 0xf74e0), and Forced Missions.txt (0x2029b0, at 0xf6e3f) forces some. This answers docs/not-implemented.md's 'which script and briefing text goes with a territory is not known'. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A territory's battle is picked from the mission files as the original picks it
- [ ] #2 Its briefing, debriefing and allies are the file's
- [ ] #3 The sub-houses' opinions change with each outcome and decide who allies with the player
<!-- AC:END -->
