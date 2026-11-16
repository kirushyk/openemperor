---
id: TASK-17
title: Build walls as the original does
status: In Progress
assignee: []
created_date: '2026-11-16T01:00:00Z'
updated_date: '2026-11-16T01:00:00Z'
labels: []
dependencies: []
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Walls are built from the Buildings tab. Picking a wall makes the cursor one cell; a click sets the start, right click or a two-finger click cancels. While the start is set, the route the wall would take is highlighted one cell wide, as straight as possible, going round ground that can't be built on, buildings and units. Walls then build one piece after another with the clockwise sweep on the icon, as for infantry and vehicles. Each piece looks as the original's: alone, an end, a corner, straight, a T or a cross, with each house's own models. Logic is taken from the original's executable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Walls can be picked in the Buildings tab where the original offers them
- [ ] #2 A click sets the start, right click or two-finger click cancels it, and the route shown follows the original's rules
- [ ] #3 Walls build one piece after another with the sweep on the icon
- [ ] #4 Each piece uses the house's model for its shape: alone, end, corner, straight, T or cross
- [ ] #5 Tests cover the route, the building and the shapes
<!-- AC:END -->
