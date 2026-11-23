---
id: TASK-161
title: Save and load games
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 161000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original saves to .igs files (%s.igs at file 0x1f1efc, used at 0x8f584) in SAVED_GAMES (0x1ef9bc): the world (CWorld::Save/Load, 0x2264c8 and 0x2264d8, used at 0x19700e and 0x19722d), the AI's sides, the campaign (CAMPAIGN_DATA 0x1f1d30, MISSION_DATA 0x1f1d20) and autosaves named for the chapter (AutosaveChap1 ... AutosaveEndGame, 0x1f1e30-0x1f1f1c). The load/save screen is LoadSave (LoadSaveSave 0x1fd380, used at 0xe35b2). Our Load Game and Save Game buttons do nothing. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A battle can be saved and loaded back as it was
- [ ] #2 A campaign can be saved, autosaves at each chapter, and can be resumed
- [ ] #3 The load/save screen lists, saves, loads and deletes saves
<!-- AC:END -->
