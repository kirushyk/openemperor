---
id: TASK-181
title: A compiler and decompiler for the mission script language
status: To Do
assignee: []
created_date: '2026-11-23T17:59:00Z'
labels: []
dependencies: []
ordinal: 370000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's missions are scripts written as text and shipped only tokenised: 229 .tok files in MISSIONS0001.RFD. missionlist.ini still names their sources (ATStart.txt and so on), but no source text is on the disc. docs/tok-format.md describes the tokens, and src/mission.rs reads and runs them.

GAME.EXE names all 181 tokens in its table at 0x1fe0b0 (records of 0x5c bytes):
- F0 to F127, and K128 to K161, are functions (ModelTick, Random, … SetVeterancy, BreakPoint, NormalConditionLose);
- K162 int, K163 obj and K164 pos declare variables;
- K165 if, K166 else and K167 endif;
- K168 to K175 are ==, !=, >=, <=, >, <, && and ||;
- K176 FALSE and K177 TRUE;
- K178 +, K179 - and K180 =.

So the language can be written back in the original's own words.

Its MissionParser checked each script as it loaded. Its messages, 'ParserError in %s, Invalid line %d, %s' (0x202378), with their checks at 0xeec30 to 0xf59d0, say which rules a script had to keep.

This group makes a decompiler, .tok to readable text, and a compiler, text to a .tok the original's parser would accept, with the original's checks. Together they make the missions readable and changeable, and new ones possible. The tool (TASK-179.7), the console (TASK-180.2) and the script debugger (TASK-180.8) use them.

Where the code goes: beside mission.rs's reader, in mission.rs or in a module of its own if that is the proper place; not in the tool's binary. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory (docs/game-exe.md).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All its subtasks are done
- [ ] #2 Every one of the 229 scripts decompiles, and compiles back byte for byte
<!-- AC:END -->
