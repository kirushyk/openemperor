---
id: TASK-179.7
title: 'Mission scripts: .tok to text and back'
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-179
ordinal: 358000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
docs/tok-format.md describes the scripts' byte code, and mission.rs runs it. GAME.EXE's own table names its tokens: 181 of them, the functions F0 to F127 and the keywords K128 to K180. MISSIONS0001 has 229 .tok and missionlist.ini.

Commands:
- `convert x.tok -o x.txt` writes the script as text: functions and keywords by their names, variables by their kind (int, object, position) and number, literals, if/else/endif, and the events it waits on, laid out with indents.
- `convert x.txt -o x.tok` compiles the text back, with errors given by line.
- `info x.tok` lists the functions it calls and how often, and the variables it uses.

All 229 come back byte for byte. That makes the missions readable and changeable, and the script runner of TASK-180.8 shows scripts this way.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every one of the 229 scripts converts to text and back byte for byte
- [ ] #2 The text uses the original's names for every function and keyword
- [ ] #3 A mistake in the text gives an error naming its line, not a panic
- [ ] #4 A changed script, put in an override folder, plays in the game
<!-- AC:END -->
