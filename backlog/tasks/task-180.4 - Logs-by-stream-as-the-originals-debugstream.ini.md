---
id: TASK-180.4
title: Logs by stream, as the original's debugstream.ini
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-180
ordinal: 365000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Name the log streams: ai, money, orders, script, sound, path, render, files and sync. Each can be turned on or off:
- in a debugstream.ini of ENABLE and DISABLE lines, as the original's was (read at 0xa7660);
- with --log ai,script;
- with the console's `log` command.

They are written to a logs folder, one file a run, named by date and time as the original's were ('%s\%s%08d.log', 'logs\%02d!%02d!%d[%02d;%02d]OOS', 0xa7510).

What goes in which stream:
- The scripts' Message (F45) and TimerMessage (F47), and BreakPoint (K160), go to the script stream as well as the screen.
- The AI's decisions go to ai and money, as the original's 'Money Debug Output' (0x39470) and 'Ai General Debug' (0x6a9a0).
- The resource layer (TASK-178.2) logs to files what it opened and from where.
- sync holds the out-of-sync dumps for TASK-177.

Without geek mode, only warnings and errors are written. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory (docs/game-exe.md).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each stream can be turned on and off by file, option and console, and writes where it says
- [ ] #2 The script stream shows each Message, TimerMessage and BreakPoint of a mission as it runs
- [ ] #3 With every stream off, nothing is written and the game is no slower
<!-- AC:END -->
