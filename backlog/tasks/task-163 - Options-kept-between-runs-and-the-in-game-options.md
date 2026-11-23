---
id: TASK-163
title: Options kept between runs, and the in-game options
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 163000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original keeps its options in the registry under Software\Westwood\Emperor (0x1f3a5c) and EMPEROR.INI (0x1f5944): GameSpeed, ToolTips, MouseSensitivity, ScrollSpeed, SoundEnabled, MusicVol, EffectsVol, VoiceVol, the detail levels and screen (0x1f813c-0x1f8ca0), with COptionsStorage (0x20cfa8, used at 0x10f9d4). In battle its options screen (OptionsIG..., 0x20d0e4-0x20d3d4) has restart, abort mission and the message history. Ours are not kept between runs. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Options are kept between runs
- [ ] #2 The in-game options offer restart, abort and the message history as the original's
<!-- AC:END -->
