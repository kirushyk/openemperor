---
id: TASK-179.7
title: 'Mission scripts: .tok to text and back'
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
updated_date: '2026-11-23T17:59:00Z'
labels: []
dependencies: []
parent_task_id: TASK-179
ordinal: 358000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The tool's commands for mission scripts, on top of TASK-181, which does the work:
- `oe-files convert x.tok -o x.txt`: decompile (TASK-181.2), with --names and --notes;
- `oe-files convert x.txt -o x.tok`: compile (TASK-181.3), checks included (TASK-181.4);
- `oe-files script check FILE…`: only the checks, with errors as file:line:column: message;
- `oe-files info x.tok`: the functions it calls and how often, its variables, and the kinds and speech it uses;
- `oe-files script new`, `script build` and `script functions --json` (TASK-181.5, TASK-181.6).

MISSIONS0001 has 229 .tok and missionlist.ini.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every one of the 229 scripts converts to text and back byte for byte
- [ ] #2 The text uses the original's names for every function and keyword
- [ ] #3 A mistake in the text gives an error naming its line, not a panic
- [ ] #4 A changed script, put in an override folder, plays in the game
<!-- AC:END -->
