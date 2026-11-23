---
id: TASK-160.2
title: Original's CAiPatternSpecialWeapon::DoResults
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-160
ordinal: 324000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ CAiPatternSpecialWeapon::DoResults, in GAME.EXE at file offset 0x40020, 0x27c bytes (+0x400000 in memory; the size runs to the next known function start). What it does: When the superweapon is charged, picks a target square from what it knows of the enemy and fires at it ('Special Weapon charged - firing!'; 'No target in tile!'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what CAiPatternSpecialWeapon::DoResults does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
