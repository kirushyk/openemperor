---
id: TASK-165
title: Health halos from the original's script
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 165000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Halos and bars are laid out by health\script.txt (file 0x202ff0, read at 0xf8ac9; parser messages 0x1f70b8 at 0xc8831): GuardShieldSize, GroupTextSize, HaloSpeed (0x1f70e0-0x1f7100), with pictures %s\health\%s.tga and %s%d.tga for Health, Veteran, Shield, Carry and Advantage (0x1f7140-0x1f7198). Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Selected units' halos, bars, ranks, group numbers and shields are drawn as the script lays them out
<!-- AC:END -->
