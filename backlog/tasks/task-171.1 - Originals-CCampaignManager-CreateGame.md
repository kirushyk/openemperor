---
id: TASK-171.1
title: Original's CCampaignManager::CreateGame
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-171
ordinal: 339000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CCampaignManager::CreateGame, in GAME.EXE at file offset 0x90260, 0x215 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Sets up a campaign battle for territory T attacked from another ('T%d attacked from T%d'), with the side of each house. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CCampaignManager::CreateGame does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
