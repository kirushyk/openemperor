---
id: TASK-158.3
title: Original's AiIni (ai_difficulty.ini)
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 283000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ AiIni (ai_difficulty.ini), in GAME.EXE at file offset 0x31aa0, 0x197 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Loads ai_difficulty.ini: a block per tech level ('AiIni: Setting difficulty for tech level %d') with a level 1-9 ('expecting level from 1 to 9 got %d') that scales the AI's figures. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what AiIni (ai_difficulty.ini) does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
