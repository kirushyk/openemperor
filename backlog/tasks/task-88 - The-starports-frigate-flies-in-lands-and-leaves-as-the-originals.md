---
id: TASK-88
title: The starport's frigate flies in, lands and leaves as the original's
status: Done
assignee: []
created_date: '2026-11-20T21:11:00Z'
updated_date: '2026-11-20T21:11:00Z'
labels: []
dependencies: []
ordinal: 88000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User: 'when starport frigate arrives, add actual frigate in a way original game does'. The rules' [Frigate] (Speed 12, TurnRate 0.1, HeightOffset 50), its model G_Frigate with its sequences Fly, FlyToHover, Hover, Land, Stationary, Takeoff, HoverToFly, letting its cargo out one at a time.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The frigate sets out from the map edge nearest the starport so as to be down as FrigateCountdown runs out
- [x] #2 It slows to a hover over the starport's deck, comes down, sets its legs down and stands facing the starport's way
- [x] #3 It lets out what it brings one at a time by the starport's door, then takes off and flies off the map
- [x] #4 It is drawn from the disc's model, each stage by its own sequence
- [x] #5 With the starport gone it turns back and what it brings is paid back
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
units::Frigate flies the visit; game.rs sends it so it lands as the countdown ends and lets cargo out through deliver_at; render/original.rs draws Units\G_Frigate_H0.xbf by stage. From the models: the starports' deck is 8 high, centred 16 before the middle; the frigate's legs reach 27.3 below it. Guessed, not from the original: one unit let out a second, coming from and leaving by the nearest edge, the climb speed of other aircraft. Its own sounds (FrigateLand, FrigateTakeOff, ATFrigateMoving) are not played yet; without the disc it is not drawn.
<!-- SECTION:FINAL_SUMMARY:END -->
