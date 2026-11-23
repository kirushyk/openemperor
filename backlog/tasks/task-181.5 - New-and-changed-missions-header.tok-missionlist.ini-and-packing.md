---
id: TASK-181.5
title: 'New and changed missions: header.tok, missionlist.ini and packing'
status: To Do
assignee: []
created_date: '2026-11-23T17:59:00Z'
labels: []
dependencies: []
parent_task_id: TASK-181
ordinal: 375000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A script alone is not a mission the game can list. MISSIONS0001 also has:
- **missionlist.ini**, which names each script's source (ATStart.txt);
- **header.tok**, a 228-byte record for each script, read at 0xf6d80 beside 'Forced Missions.txt'.

What the 228 bytes hold must be found: from the loader at 0xf6d80, and by comparing the records with the scripts they belong to. The territory screen also needs a briefing, a mini briefing and a win debrief in the strings: 'Mission %s is missing … in strings.txt', 0xf6e92 to 0xf6f30.

Commands:
- `oe-files script new NAME`: writes a source to start from, with a header entry and a missionlist line, and empty briefing strings.
- `oe-files script build DIR`: compiles every source, writes header.tok and missionlist.ini, and lays the result out as an override folder (TASK-178.5) or packs it into a MISSIONS archive (TASK-179.2).
- `--missions` and `--mission NAME` find a mission added this way. Missions added to the campaign's map go through TASK-171's files. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory (docs/game-exe.md).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 What header.tok's records hold is written in docs/tok-format.md from the original's code
- [ ] #2 A mission written from scratch is built, listed by --missions and played with --mission, with its briefing
- [ ] #3 Building the 229 originals' sources gives back header.tok and missionlist.ini byte for byte
<!-- AC:END -->
