---
id: TASK-47
title: Place each map's own objects
status: Done
assignee: []
created_date: '2026-11-18T11:58:00Z'
updated_date: '2026-11-18T15:09:00Z'
labels: []
dependencies: []
ordinal: 47000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A map's test.xbf has a part 0xa0000007 listing objects put on the map itself: name, cell x (u16), group (u16), cell y (u32), facing (u32). The Atreides first map has a walled Harkonnen compound mid-map (77 HKWall, an HKFactoryFrigate, money crates, barrels, two GPINSpotlight), Sardaukar crates, wrecks, hanging figures and green smoke. None is placed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Map objects are read from part 0xa0000007
- [x] #2 Walls, the factory frigate, spotlights, wrecks, figures and smoke stand on the map with the disc's models
- [x] #3 Money and Sardaukar crates lie where the map has them
- [x] #4 The Atreides first battle shows its mid-map compound
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
6ddb242, 673e095; d3296e2 (crates of units, each kind's model); scenery (60 kinds from rules/art) and AT/OR frigates. Left out, needing units or buildings we lack: ATMilitia, ATSonicTank, ATGeneral, FRADVFremen, ORAATrooper, ORDeviator, GUNIAP, INYak(Hauder), GUMegaCannon, IXMegaCannon, GUbubble, GUpyramid, IXWall, LeechCrate.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
All map objects the rules give as incidentals, crates and frigates are placed with disc models; the remaining kinds need roles we lack.
<!-- SECTION:FINAL_SUMMARY:END -->
