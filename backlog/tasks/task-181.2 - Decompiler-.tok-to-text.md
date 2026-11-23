---
id: TASK-181.2
title: 'Decompiler: .tok to text'
status: To Do
assignee: []
created_date: '2026-11-23T17:59:00Z'
labels: []
dependencies: []
parent_task_id: TASK-181
ordinal: 372000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Turn a .tok into the text of TASK-181.1.

- One text line for each line of the .tok. Empty lines stay empty, so line numbers match the original's and the parser's errors point at the right line.
- Functions and keywords by their GAME.EXE names. Kinds of unit and building by their Rules.txt names. Speech by its ids. Each comes from the discs' own files, Rules.txt and E_Output_Pickup.txt, through the resource layer (TASK-178.2), or from those given to it.
- Variables are named by their type and place: `int i0`, `obj o3`, `pos p5`. `--names FILE` gives better names, one `old new` pair a line, so that a reader's names can be kept from one decompile to the next.
- Lines inside an if are indented.
- With `--notes`, a speech line has the English line's words beside it as a comment, and a kind has its house beside it.
- A token or byte it does not know is kept in a form that compiles back to the same byte (TASK-181.1), with a warning naming the line. It never fails on any of the 229.

mission.rs's reader (read, tokens) gives the tokens; use it, don't write a second reader.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every one of the 229 scripts decompiles with no unknown token left raw, beyond the kinds and speech numbers TASK-181.1 names as unknown
- [ ] #2 Line numbers of the text match the .tok's
- [ ] #3 --names and --notes work as described
- [ ] #4 Tests decompile small scripts built in memory and check the text
<!-- AC:END -->
