---
id: TASK-180.7
title: 'Map editor: the ground'
status: To Do
assignee: []
created_date: '2026-11-23T17:28:00Z'
labels: []
dependencies: []
parent_task_id: TASK-180
ordinal: 368000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Change a map's ground:
- raise, lower, flatten and smooth its heights, in test.xbf's ground mesh;
- paint cell types: sand, rock, cliff, where buildings can go, where nothing can pass (test.CPF);
- paint the wash (test.CPT) and texture.dat, once TASK-179.6 knows it;
- set the light (test.lit) and the size (map.inf).

Also:
- make a new map of a given size from scratch;
- remake its radar thumbnails;
- have what the battlefield works out from the ground (slopes, cliffs, the islands that can be built on) worked out again at once.

Every file is written in the original's format, through writers added to xbf.rs and to TASK-179.6's code, so the map can be packed into a MAPS archive as the original's are.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each change above can be made and shows at once, with undo
- [ ] #2 A new map made from scratch, with start places from TASK-180.6, plays a skirmish
- [ ] #3 Changed maps can be saved, converted (TASK-179.6) and packed (TASK-179.2) like the original's
<!-- AC:END -->
