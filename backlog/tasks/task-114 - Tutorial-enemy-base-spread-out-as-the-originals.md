---
id: TASK-114
title: Tutorial enemy base spread out as the original's
status: Done
assignee: []
created_date: '2026-11-21T20:57:00Z'
updated_date: '2026-11-21T21:05:00Z'
labels: []
dependencies: []
ordinal: 114000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The script brings the enemy's refinery, windtrap, yard and barracks with F21 at one point, and ours stands them all on it, overlapping.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Buildings a script brings each stand on free squares near the point, none overlapping
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
What a script brings with F21 (no facing) takes Anchor::Nearest: free_plot_near searches ring by ring for the nearest squares on rock (else any walkable ground) clear of other buildings by 2 squares. The tutorial's Harkonnen refinery, yard, windtrap and barracks now stand apart on their rock.
<!-- SECTION:FINAL_SUMMARY:END -->
