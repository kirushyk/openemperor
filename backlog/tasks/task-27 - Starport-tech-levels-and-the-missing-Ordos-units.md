---
id: TASK-27
title: Starport, tech levels and the missing Ordos units
status: To Do
assignee: []
created_date: '2026-11-16T21:33:00Z'
updated_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From docs/not-implemented.md: starports (the palace needs one), TechLevel gating, the Ordos AA trooper, sonic tank and deviator.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each is built or gated as Rules.txt says, with tests
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
GAME.EXE (file offsets; +0x400000 for the address; map in docs/game-exe.md): TechLevel (rules key record 0x215e5c), UpgradeTechLevel (0x21581c), ai.ini's FirstCampaignGameTechLevel (0x1e59d8); the skirmish's GameSetupTechLevelEdit (0x224098); starport data ('Failed to find starport data entry' 0x220a18, used at 0x13bcd6) and the frigate's FrigateCountdown (0x21009c).
<!-- SECTION:NOTES:END -->
