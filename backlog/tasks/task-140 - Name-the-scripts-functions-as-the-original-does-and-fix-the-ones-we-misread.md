---
id: TASK-140
title: Name the script's functions as the original does, and fix the ones we misread
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
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
- [ ] #1 mission.rs names its function and keyword constants after the original's names
- [ ] #2 F2, F11, F16, F27, F32, F42, F114, F115 and F135 do what their names and case code say, or the doc says why not
- [ ] #3 Tests cover the corrected functions and pass
<!-- AC:END -->
