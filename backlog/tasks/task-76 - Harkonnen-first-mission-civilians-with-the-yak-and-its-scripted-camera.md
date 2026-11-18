---
id: TASK-76
title: 'Harkonnen first mission: civilians with the yak, and its scripted camera'
status: Done
assignee: []
created_date: '2026-11-18T18:28:00Z'
updated_date: '2026-11-18T19:21:00Z'
labels: []
dependencies: []
ordinal: 76000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User: 'when playing first mission for harkonnen, civilians with yak should be present. also implement this scripted camera move and rotation'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Harkonnen first mission has its civilians and yak as on the disc
- [x] #2 The mission script's camera moves and turns play
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Camera done: K131/K132 scene, F100/F114 look, F101/F115 glide, F105 circle (2 degrees an update = 20 degrees a second, once round in 18 s), F106 stop, F112 keep, F113 glide back. Way 1 is taken as positive yaw: a guess. Civilians: the Harkonnen first map has INYak and INYakHauder objects, not yet known as roles.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Camera scenes played (7d32f58); the yaks and the yak hauler now stand on the Harkonnen first map as nobody's (task 81).
<!-- SECTION:FINAL_SUMMARY:END -->
