---
id: TASK-28
title: Low power as the original has it
status: To Do
assignee: []
created_date: '2026-11-16T21:33:00Z'
updated_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Find out what the original's low power does to build speed, radar and turrets, in place of our guess.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Our low-power rules follow the original, or what is not known is logged
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
GAME.EXE (file offsets; +0x400000 for the address; map in docs/game-exe.md): PowerUp/PowerDown (0x1f773c, 0x1f7744; PowerDown used at 0xcfcd9), the gauge's Power%02d meshes (0x1f76c0, at 0xcd0fa), DisableWithLowPower (rules record 0x2166f4), SoundRadarOn/Off (0x21b384, 0x21b44c).
<!-- SECTION:NOTES:END -->
