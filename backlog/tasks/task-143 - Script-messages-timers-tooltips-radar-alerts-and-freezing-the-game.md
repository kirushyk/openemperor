---
id: TASK-143
title: Script messages, timers, tooltips, radar alerts and freezing the game
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 143000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Not carried out: GiftingMessage F46 (case file 0xf01cd, 0x33 bytes), TimerMessage F47 (20 uses; 0xf0200, 0x99) and TimerMessageRemove F48 (0xf0299, 0x26), ObjectToolTip F58 (273/30; 0xf3300, 0x2f), RadarEnabled F95 (0xf02bf, 0xe6), RadarAlert F96 (619 uses in 149 scripts; 0xf03a5, 0x27), FreezeGame F129 and UnFreezeGame F130 (69/58; 0xf3a5b and 0xf3a6d, 0x12 each), EndGameLose F85 (32/17; 0xf31c9, 0x16) and NormalConditionLose F161 (0xf441b, 0x77). The radar alert's look is the radar script's AlertMesh (Radar\%s.ini, file 0x20de44). Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A timer message counts down on the screen and is removed by TimerMessageRemove
- [ ] #2 RadarAlert flashes the minimap where the script says, and RadarEnabled turns the minimap off and on
- [ ] #3 FreezeGame stops the world while the script runs on, and UnFreezeGame lets it go
- [ ] #4 EndGameLose loses the battle and NormalConditionLose sets whether losing everything loses it
<!-- AC:END -->
