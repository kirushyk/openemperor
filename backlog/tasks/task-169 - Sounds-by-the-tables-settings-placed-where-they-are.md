---
id: TASK-169
title: Sounds by the tables' settings, placed where they are
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 169000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The sound tables (sfx.txt 0x1ef370, music.txt, dialog.txt, read at 0x7721e) take Range, Priority, Type, MinVolume, VShift, FShift, Attack, Decay, Delay, LoopCount, Limit and Control (messages 0x220434-0x22094c), with types and flags SHROUDED, NOISE_SHY, GUN_SHY, PLAYER, CENTER, SCREEN, GLOBAL, QUIET, MOVEMENT, VIOLENT, AMBIENT, CRITICAL (0x1ee820-0x1ee8f0). We read Volume and Limit; every sound plays centred. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Sounds fade with distance and pan as Range and the flags say
- [ ] #2 Priority, shifts, delay and loop count are kept as the tables give them
<!-- AC:END -->
