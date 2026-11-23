---
id: TASK-143.12
title: Script function F160 BreakPoint
status: Done
assignee: []
created_date: '2026-11-23T15:23:00Z'
updated_date: '2026-11-23T18:00:00Z'
labels: []
dependencies: []
parent_task_id: TASK-143
ordinal: 279000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf4407, 0x14 bytes; +0x400000 in memory): BreakPoint(): unused; writes 'Mission BreakPoint Hit .... CameronR' to debug channel 0xa8; does nothing else.  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's BreakPoint does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls BreakPoint and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
BreakPoint is carried out as the original's: nothing happens (its debugger line is left out), and it is not warned of as unknown. Tested in mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
