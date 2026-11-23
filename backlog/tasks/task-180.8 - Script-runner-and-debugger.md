---
id: TASK-180.8
title: Script runner and debugger
status: To Do
assignee: []
created_date: '2026-11-23T17:28:00Z'
labels: []
dependencies: []
parent_task_id: TASK-180
ordinal: 369000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Run any mission's script on its own map or another, in geek mode, with:
- a pause at its start;
- stepping by statement or by tick;
- breakpoints on lines of its text (TASK-179.7), and the script's own BreakPoint keyword (K160; the original prints 'Mission BreakPoint Hit .... CameronR', 0xf421f);
- its variables (ints, objects, positions) shown as they change, with the objects and positions they hold marked on the battlefield;
- its event list (delivered, constructed, attacked: script +0x2c0, as docs/tok-format.md has it);
- which of its conditions hold at that moment.

A script changed as text can be reloaded without restarting the map: its variables are kept if it has the same ones, and reset otherwise. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory (docs/game-exe.md).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Any of the 229 scripts runs in the debugger on its own map
- [ ] #2 Stepping, breakpoints (its own and K160), variables and events work as described
- [ ] #3 A script edited as text reloads in the running battle
<!-- AC:END -->
