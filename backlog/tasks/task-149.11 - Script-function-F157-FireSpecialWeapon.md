---
id: TASK-149.11
title: Script function F157 FireSpecialWeapon
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-149
ordinal: 276000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf4343, 0x47 bytes; +0x400000 in memory): FireSpecialWeapon(side, weapon, pos): unused; issues an order (0x4c5090 -> order queue 0x4c55b0) to fire the side's superweapon at pos, the weapon chosen by building kind: ATHawkWeapon, ORBeamWeapon or HKDeathHand ('Unknown special weapon index' otherwise).  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's FireSpecialWeapon does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls FireSpecialWeapon and checks the effect
<!-- AC:END -->
