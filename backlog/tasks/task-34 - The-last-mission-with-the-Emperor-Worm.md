---
id: TASK-34
title: The last mission with the Emperor Worm
status: To Do
assignee: []
created_date: '2026-11-17T07:34:00Z'
updated_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The campaign's end mission (#e1 end mission emperor worm, ATENDMission.tok and the others) includes the Emperor Worm itself.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The end mission is played on its map with the Emperor Worm in it, as the original's script has it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
GAME.EXE (file offsets; +0x400000 for the address; map in docs/game-exe.md): the giant worm is unitBigWorm.cpp (path 0x222760, used at 0x166ee8); BigWorm in the rules' keyword table at 0x218f94; the AI's CAiVerticalWorm (0x1eda38) and BigWorm (0x1eda30) names.
<!-- SECTION:NOTES:END -->
