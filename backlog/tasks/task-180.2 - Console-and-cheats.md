---
id: TASK-180.2
title: Console and cheats
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-180
ordinal: 363000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A console opened with a key (` by default, in TASK-162's bindings), with history and completion.

Its commands:
- reveal; fog on|off
- cash SIDE N
- spawn KIND [SIDE] [N], at the cursor
- kill, heal and god (cannot be hurt), on the selected units
- rank N, for veterancy
- side N, giving the selected to a side
- build instant; power infinite
- pause; step [TICKS]; speed X
- ai SIDE on|off
- win; lose
- goto X Y; camera free
- help

It also runs every mission-script function by its original name and arguments, through the same code mission.rs uses: SetSideCash 1 5000, NewObject …, CarryAllDelivery …. So each function of TASK-141 to TASK-149 becomes a console command as soon as it is done. Objects and positions are given by the selected unit, the cursor, or numbers.

Cheats work only in geek mode, and mark the game (see TASK-180).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The console opens in battle in geek mode, and every command above works
- [ ] #2 Every script function our mission.rs runs can be called from it by name
- [ ] #3 A wrong command or argument gives a message, not a panic
- [ ] #4 Tests run commands against a battle and check their effect
<!-- AC:END -->
