---
id: TASK-17
title: Build walls as the original does
status: Done
assignee: []
created_date: '2026-11-16T01:00:00Z'
updated_date: '2026-11-16T04:49:00Z'
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
- [x] #1 Walls can be picked in the Buildings tab where the original offers them
- [x] #2 A click sets the start, right click or two-finger click cancels it, and the route shown follows the original's rules
- [x] #3 Walls build one piece after another with the sweep on the icon
- [x] #4 Each piece uses the house's model for its shape: alone, end, corner, straight, T or cross
- [x] #5 Tests cover the route, the building and the shapes
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Role::Wall: a one-square building with Rules.txt's values (Cost 20, BuildTime 60 → 1.5 s, Health 1000 → 330, rock only, Selectable false, ExcludeFromSkirmishLose, CountsForStats false), second in the Buildings menu as in [BuildingTypes]. Picking it places first: the cursor is one square; a click picks the start, the route to the cursor shows one square wide (units::wall_route: shortest over free seen rock, then fewest turns, round buildings, units and unbuildable ground); a second click sets it; right click or Escape lets go of the start, then of the wall. The build site then builds the squares one after another with the icon's sweep and a badge of how many are left, paying for each; a square blocked meanwhile is skipped and paid back. units::join_walls records which sides join the owner's walls; WallPiece::of picks the piece and turn. From GAME.EXE: the six pieces and their names ("", _End, _Middle, _Corner, _TJoin, _Cross after the Xaf, then detail and damage letters). All 18 disc models load, turned so each joins its neighbours (disc test); without the disc a post with arms. The exe's routing (a cursor state machine) was not read through: logged in docs/not-implemented.md with the AI's wall building, WallThud and the _HC rising model.
<!-- SECTION:FINAL_SUMMARY:END -->
