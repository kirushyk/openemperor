---
id: TASK-148
title: Picture-in-picture camera
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 148000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original shows a second, small view for the scripts: PIPCameraLookAtPoint F114 (108 uses; case file 0xf3760, 0x4d), PIPCameraPanToPoint F115, PIPCameraZoomTo F117 (0xf383d), PIPCameraStartRotate/StopRotate F119/F120 (0xf389b, 0xf38ca), PIPCameraTrackObject F121 (435 uses in 136 scripts; 0xf38ef, 0x2d), PIPCameraStore/Restore F126/F127 and PIPRelease F128 (466 uses; 0xf3a23, 0x38). We send F114 and F115 to the main camera and drop the rest. The window's place and frame are in the in-game interface scripts (UI data). Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A script's PIP calls open a small view showing what they point it at, and PIPRelease closes it
- [ ] #2 The main camera is left alone by them
- [ ] #3 Its place and size are the original's
<!-- AC:END -->
