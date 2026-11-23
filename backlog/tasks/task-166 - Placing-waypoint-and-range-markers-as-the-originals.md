---
id: TASK-166
title: Placing, waypoint and range markers as the original's
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 166000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's placing models: placement\build_arrow, build_wall, build_skirt, build_cantbuild, build_building (.xbf, file 0x222cbc-0x222d30, used at 0x1763ee and 0x17618b); waypoints\WP_%s_H0.XBF (0x222de8, used at 0x1789d1) with Waypoint_Selected, Waypoint_Small, Anchor and Anchor_Selected, VectorChevron.tga and waypointchevron.tga (0x222d74-0x222dd0); target lines (ShowTargetLines 0x1f6cc8, used at 0xc076a); range indicators (RangeIndicator/RangeMask 0x21e96c/0x21ea34, systemrange_32.tga 0x20e080 at 0x11d5c1). Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Placing a building or wall shows the original's models
- [ ] #2 Waypoints and target lines are drawn as the original's
- [ ] #3 Selected turrets show their range as the original's
<!-- AC:END -->
