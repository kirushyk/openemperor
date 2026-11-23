---
id: TASK-178
title: Game resources a player can find their way around
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
ordinal: 345000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The game's data come on four discs, nested in containers:
- a Microsoft cabinet, INSTALL/Game1.cab, which holds the archive indexes (*.RFH), the smaller archives' data and some loose files;
- RFH/RFD archives, with their index in the cabinet and their data loose on the disc, or both in the cabinet;
- GABA sound bags;
- Bink movies, loose and inside MOVIES0001.RFD.

Today the game finds them only as discs mounted under /Volumes/EMPEROR1 to EMPEROR4, and only on macOS (src/discs.rs, and DISC in src/main.rs, behind cfg(target_os = "macos")). Each module joins its own paths onto the disc: maps.rs, sounds.rs, campaign.rs, music.rs, video.rs and render/original.rs.

This group gives one place where any file can be found by name, whether it comes from discs, folders or disc images. It documents every kind of file, and lets loose files take the place of archived ones, as the original's resource folders allow. The command-line tool (TASK-179) and geek mode (TASK-180) build on it. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory (docs/game-exe.md).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All its subtasks are done
<!-- AC:END -->
