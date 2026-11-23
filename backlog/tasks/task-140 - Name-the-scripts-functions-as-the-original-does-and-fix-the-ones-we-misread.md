---
id: TASK-140
title: Name the script's functions as the original does, and fix the ones we misread
status: In Progress
assignee: []
created_date: '2026-11-23T14:32:00Z'
updated_date: '2026-11-23T18:18:00Z'
labels: []
dependencies: []
ordinal: 140000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
GAME.EXE names every function and keyword of the mission scripts: a table at file 0x1fe0b0-0x2021bc, 181 records of 0x5c bytes, each a name, its place in the table being its token number (F0 ModelTick ... K180 '='). The interpreter is one switch at file 0xf0011 (cmp ecx,0xb1; jmp [4*ecx+0x4f4a00]); its jump table is at file 0xf4a00, 178 dwords (0x2c8 bytes), and docs/game-exe.md lists each function's case code and size. Several of our readings in src/mission.rs disagree with the names (case code at file offset, size): F2 Multiplayer (ours READY; 0xf0182, 0x27), F11 GetEntranceFarFromPos (ours NEAR; 0xf07cb, 0x55), F16 GetEnemySide (ours SOMEONE, a world-at-large side; 0xf0e82, 0xb6), F27 ObjectVisibleToSide (ours HURT_BY; 0xf090a, 0xec), F32 SideNearToSide (ours SEES; 0xf1b0c, 0x195), F42 EventSideAttacksSide (ours FOUGHT; 0xf30f3, 0x57), F114/F115 PIPCameraLookAtPoint/PanToPoint (ours the main camera, LOOK_AT_TOO/GLIDE_TO_TOO; 0xf3760, 0x4d and 0xf37ad, 0x48), F135 ObjectSell (ours REMOVE; 0xf3edb, 0x75; F136 ObjectRemove is 0xf3b85, 0x2c). Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 mission.rs names its function and keyword constants after the original's names
- [ ] #2 F2, F11, F16, F27, F32, F42, F114, F115 and F135 do what their names and case code say, or the doc says why not
- [ ] #3 Tests cover the corrected functions and pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
What the original's case code does, for the functions we misread:
- F2 Multiplayer() (0xf0182, 0x27): TRUE only when the game type (gSetup 0x7c86dc, 0x533fc0) is 3, the co-op campaign; FALSE in a single-player battle. We take it as true.
- F11 GetEntranceFarFromPos(pos) (0xf07cb, 0x55; core 0x4ecca0): of the map's entrances (map data 0x44a520, entrance list 0x4b5980, count 0x45ff70), the one farthest from pos. We take it as the place itself.
- F16 GetEnemySide() (0xf0e82, 0xb6): the first of the 16 sides that exists and whose set-up slot is not a human player's (0x5351c0: the slot's byte +0x32 is 0), -1 if none: the first computer side. We take it as a 'world at large' side.
- F27 ObjectVisibleToSide(obj, side) (0xf090a, 0xec): TRUE if side's bit is set in the object's seen-by mask (+0x6c); co-op expansion on side. We take it as 'hurt by'.
- F32 SideNearToSide(a, b) (0xf1b0c, 0x195; core 0x4edba0): TRUE if any unit or building of one side stands within round(size/2)+5 squares of an object of the other and is seen by it; computed once per update per pair (masks 0x89c2d4 / 0x89c354). We take it as 'in sight of'.
- F42 EventSideAttacksSide(a, b) (0xf30f3, 0x57; 0x4f5cc0, co-op 0x4f5d90): TRUE if this update's event list (script+0x2c0) has a kind-11 attack event by an object of side a on side b. We take it as 'has lost any of what the script gave it'.
- F114/F115 PIPCameraLookAtPoint/PanToPoint (0xf3760, 0xf37ad): drive the picture-in-picture window's own camera (created on first use, 'GreenTVScreenOn'), never the main one.
- F135 ObjectSell(obj) (0xf3edb, 0x75): sells the building (Building 0x486660, the sell routine of building.cpp: it refunds and goes away as a player's sale does) when the object can be sold (its virtual +0x14, or game type 4 with the side's 0x434320 state allowing it); otherwise logs 'eObjectSellFunction: ObjectCan'tBeSold %d'. ObjectRemove (F136) is the one that takes a thing off with order 0x2e.

F11 GetEntranceFarFromPos now gives the farthest of the map's entrances; F6 GetEntrancePoint was also misread (it is the entrance nearest the side's mission position, 0x4ecd90 -> 0x4ecb70) and is fixed with it.

K135 ObjectSell now sells a building for its owner (half its cost, no words), and leaves what is not a building; K136 ObjectRemove takes things off the map.

Read from the code (2026-11-23): F32 SideNearToSide(a, b) (core 0x4edba0) walks the side with fewer objects (counts by 0x53f300 kinds 1 and 2) and asks 0x4ed810(obj, other side) of each, caching the answer per pair in 0x89c2d4/0x89c354 for the update. 0x4ed810(obj, side) is also F24 ObjectNearToSide's core: r = round(rules Size (+0x9c) x 0.5); over the map squares within r+5 squares of the object's square (distance squared < (r+5)^2), true if an occupant of a square belongs to 'side' and its seen-by mask (+0x6c) has the object owner's bit. So F24 (ours: within REACH_OF_A_PLACE of the thing's edge, the tutorial's beacons) is 'within half its size + 5 squares, and seen by its owner', and F32 is F24 asked of each of the smaller side's things. Not changed yet: the tutorial's beacons ride on F24, and our sides keep no seen-by masks (the nearest we have is Battle::sees, by sight range).
<!-- SECTION:NOTES:END -->
