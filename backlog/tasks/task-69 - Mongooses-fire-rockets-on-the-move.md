---
id: TASK-69
title: Mongooses fire rockets on the move
status: Done
assignee: []
created_date: '2026-11-18T16:25:00Z'
updated_date: '2026-11-21T00:00:00Z'
labels: []
dependencies: []
ordinal: 69000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A Mongoose should launch its rockets while it moves.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A Mongoose fires its rockets while driving
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
It could not: a vehicle turns its whole body to aim, and walking its body follows its way. The rules give the Mongoose's launcher its own turret, ATMongooseMissile, swinging ±190° at 3° an update; Role::turret_swing says so, turn_to_aim swings its gun_turn onto the aim while it walks (and back ahead when it has none), and render/original.rs turns its model's head (::0~~1turret) by it, so its rockets leave from where the launcher points. Weapons test: it fires walking past an enemy. Other vehicles' turrets still turn with the body.
<!-- SECTION:FINAL_SUMMARY:END -->
