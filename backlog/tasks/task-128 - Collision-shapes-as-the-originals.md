---
id: TASK-128
title: Collision shapes as the original's
status: In Progress
assignee: []
created_date: '2026-11-21T23:09:00Z'
updated_date: '2026-11-21T23:31:00Z'
labels: []
dependencies: []
ordinal: 128000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Check how the original defines each unit's and building's collision (Size, the models' SLCT and bounding parts, Occupy) and make ours match.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each unit collides with the size and shape the original gives it
- [ ] #2 Each building blocks the squares and shape the original gives it
- [ ] #3 Tests cover it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Units: compared every unit's rules Size with our radius (Size x 16): all matched but IXProjector, StuntATADVCarryall and FakeHarvester (size 3, were 2), fixed. Buildings: still block their whole plot rectangle; the rules' Occupy grids mark b (blocked), n (free), d/p (decks and pads units drive on) and s (room kept clear in front). Next: block only b cells.
<!-- SECTION:NOTES:END -->
