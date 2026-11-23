---
id: TASK-180
title: 'Geek mode: sandbox, debug views and editors'
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
ordinal: 361000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A mode for people who want to look inside the game and play with it. It is turned on with --geek or an option (TASK-163); it then shows as an entry in the main menu and adds a key for the console in battle.

The original kept such things for its developers, and the player never reached them:
- a debug window and pane, drawn with bitmapfont.tga and the font maps: the pane is made at 0xa1b30 and its font surface at 0xa1bdf; the "DebugChildWindowClass" window at 0xa2270 and 0xa2379;
- log streams turned ENABLE or DISABLE by debugstream.ini (read at 0xa7660), written to logs\ as '%s\%s%08d.log', with out-of-sync dumps as 'logs\%02d!%02d!%d[%02d;%02d]OOS' (0xa7510);
- the AI's debug output: 'Money Debug Output : OFF' (0x39470) and 'Ai General Debug' (0x6a9a0);
- the scripts' BreakPoint keyword, K160, whose case prints 'Mission BreakPoint Hit .... CameronR' (0xf421f);
- WinMain's gDebugMapName (0xa8770), to start straight on a map;
- an FXEDITOR.INI left over from an interface effects editor, in UI0001.

Ours is for players. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory (docs/game-exe.md).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All its subtasks are done
- [ ] #2 Nothing in geek mode changes a campaign or skirmish played without it
- [ ] #3 A game that used a cheat or editor says so on its score screen and in its saves
<!-- AC:END -->
