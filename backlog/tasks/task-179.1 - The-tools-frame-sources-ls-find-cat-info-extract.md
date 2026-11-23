---
id: TASK-179.1
title: 'The tool''s frame: sources, ls, find, cat, info, extract'
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-179
ordinal: 352000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Make the `oe-files` binary and its commands that work on any file:
- sources
- ls, with -l and -R
- find
- cat
- info, which says only what kind a file is until the subtasks add their own
- extract

Share the readers with the game as the group's description says. Output is plain, one item a line, for scripts. -l adds size, packed size and kind. Paths that have spaces, as the maps' folders do ('#A1 JF Caladan Capital LOD2'), print quoted, as --missions prints names now.

Exit codes: 0 when done, 1 when something was not found, 2 for a bad command line.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 oe-files builds as a second binary with no new dependency and prints to the console on macOS, Windows and Linux
- [ ] #2 sources, ls, find, cat, info and extract work on every container kind: cabinet, RFH/RFD, bag, and folder or image
- [ ] #3 `oe-files extract MAPS0001 -o out` writes every map's files under out/MAPS0001/
- [ ] #4 Tests run the commands on small containers built in memory
<!-- AC:END -->
