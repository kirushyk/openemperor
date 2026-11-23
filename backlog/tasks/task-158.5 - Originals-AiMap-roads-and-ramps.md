---
id: TASK-158.5
title: Original's AiMap (roads and ramps)
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-158
ordinal: 285000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ AiMap (roads and ramps), in GAME.EXE at file offset 0x34390, 0xe0 bytes; file offset 0x3534b, 0x1f5 bytes; file offset 0x355f0, 0xf0 bytes; file offset 0x35dc7, 0x29f bytes; file offset 0x36066, 0x29f bytes (+0x400000 in memory; the size runs to the next known function start). What it does: The AI's map of its base: it lays straight roads (never diagonal: 'AiMap: Impossible to draw diagonal road') from its buildings to the rock's ramps, in the four directions, so its buildings keep a way out. Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what AiMap (roads and ramps) does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
