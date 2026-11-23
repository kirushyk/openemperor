---
id: TASK-171.2
title: Original's CCampaignManager::SetupMissionData
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-171
ordinal: 340000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CCampaignManager::SetupMissionData, in GAME.EXE at file offset 0x90ea5, 0x8b bytes; file offset 0x91648, 0x288 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Finds the mission of the territory ('Mission Missing on Territory %d') and its briefing ('Mission %s is missing briefing in strings.txt') and sets the sides' colours for the game mode. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CCampaignManager::SetupMissionData does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
