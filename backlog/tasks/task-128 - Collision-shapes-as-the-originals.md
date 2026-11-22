---
id: TASK-128
title: Collision shapes as the original's
status: Done
assignee: []
created_date: '2026-11-21T23:09:00Z'
updated_date: '2026-11-22T00:45:00Z'
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
- [x] #1 Each unit collides with the size and shape the original gives it
- [x] #2 Each building blocks the squares and shape the original gives it
- [x] #3 Tests cover it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Units: compared every unit's rules Size with our radius (Size x 16): all matched but IXProjector, StuntATADVCarryall and FakeHarvester (size 3, were 2), fixed. Buildings: still block their whole plot rectangle; the rules' Occupy grids mark b (blocked), n (free), d/p (decks and pads units drive on) and s (room kept clear in front). Next: block only b cells.

Buildings now block only their Occupy b squares, turned as they face: units are pushed off those squares (not off a circle), steer round the nearest of them and route round each. Parking spots keep a vehicle's radius clear of b, d and p squares. Harvesters no longer stop to eat on a refinery pad's approach. Checked on the disc: the Harkonnen palace's model stands over its b squares, not their mirror. Shots and blasts still find buildings by their circle.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Unit sizes now match the rules' Size; buildings block only the squares their rules' Occupy marks b, as they face, with open, deck and pad squares free to drive on. Tests cover the grids, the facing, the push-off and the disc's palace.
<!-- SECTION:FINAL_SUMMARY:END -->
