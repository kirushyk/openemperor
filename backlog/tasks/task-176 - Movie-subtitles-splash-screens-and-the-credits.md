---
id: TASK-176
title: Movie subtitles, splash screens and the credits
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 176000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original subtitles its movies (subtitles\subtitle.ini, file 0x2214e8), shows splash pictures at start (Data\Splash\splash%02d.bmp 0x1f90c8, used at 0xd6c3b) and its Legals (0x1f9130, at 0xd73c6), has a movie picker (MoviePickerList 0x20d5e4, TitleMovies) and a credits roll (CreditsManager 0x2223c8, Credits 0x222564 at 0x15ade5). Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Movies show their subtitles when the option is on
- [ ] #2 The splash, legals, movie picker and credits are there as the original's
<!-- AC:END -->
