---
id: TASK-67
title: 'Bilar Slopes: the Atreides mission with its prison camp and lightning gates'
status: In Progress
assignee: []
created_date: '2026-11-18T16:20:00Z'
updated_date: '2026-11-18T16:46:00Z'
labels: []
dependencies: []
ordinal: 67000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Atreides campaign's Bilar Slopes mission must play with its prison camp and lightning gates as the original has them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The mission starts on Bilar Slopes with what its script and map put down
- [x] #2 The prison camp stands and works as the script uses it
- [ ] #3 The lightning gates stand and work as in the original
- [x] #4 The mission can be won and lost as its script says
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
b8eeb79: territory battles run their scripts (missionlist.ini in MISSIONS0001). Bilar Slopes (T7) for the Atreides: ATP1D7FR (prison camp: civilians guarded by an HK general and light infantry; freed as Fremen when seen with the guards dead, then won), ATP1M7FR (walled camp round an HK outpost: hurting it brings Fremen, destroying or capturing it wins), ATP2M7FR (Fremen camp), ATP2D7FR. Checked by running both phase 1 missions. No lightning gate is placed by any T7 script or by the map's own objects; HLINLighGate appears only on Harkonnen Civil War Attack and the Heighliner maps. Asked the user which they mean.
<!-- SECTION:NOTES:END -->
