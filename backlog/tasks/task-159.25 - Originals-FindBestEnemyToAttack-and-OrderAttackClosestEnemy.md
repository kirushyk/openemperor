---
id: TASK-159.25
title: Original's FindBestEnemyToAttack and OrderAttackClosestEnemy
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-159
ordinal: 320000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's C++ FindBestEnemyToAttack and OrderAttackClosestEnemy, in GAME.EXE at file offset 0x60530, 0x1d0 bytes; file offset 0x62b70, 0x80 bytes (+0x400000 in memory; the size runs to the next known function start). What it does: Chooses what a team attacks: the best visible enemy, or only 'worth' targets if the team is limited to them, else it moves on without attacking ('no visible enemies!'). Its name comes from the executable's own messages; docs/game-exe.md maps the subsystem.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game has what FindBestEnemyToAttack and OrderAttackClosestEnemy does in the original, in whatever form fits our code: it need not be one function, nor be built the same way
- [ ] #2 A test covers that behaviour
<!-- AC:END -->
