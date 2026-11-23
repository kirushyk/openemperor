---
id: TASK-181.3
title: 'Compiler: text to .tok'
status: To Do
assignee: []
created_date: '2026-11-23T17:59:00Z'
labels: []
dependencies: []
parent_task_id: TASK-181
ordinal: 373000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Turn text in the language of TASK-181.1 into a .tok.

How the file is put together:
- **Head:** the size of the rest of the file after the head's 8 bytes, then the number of lines, both as little-endian u32.
- **Lines:** each ends with a zero byte.
- **Function and keyword tokens:** 0x80 and 2 bytes. Each byte after the first carries 7 bits of the number, lowest first, with its top bit set.
- **Variable tokens:** 0x81 and 1 byte, the variable's place among the declarations.
- **Kinds:** 0x82 and 2 bytes, the Rules.txt place plus 111.
- **Speech:** 0x83 and 2 bytes, the MissionMessages place plus 502.
- Everything else stays plain text as written. Comments are dropped, and a comment-only or blank line becomes an empty line.

Names match without regard to case, as the original's table does. Check that it did, at the parser's name lookup.

Output is byte for byte what the original's tools made. A script decompiled by TASK-181.2 compiles back to the same bytes.

The checks of TASK-181.4 run before anything is written, and nothing is written if one fails. `-o` names the output; by default it goes beside the source, as NAME.tok.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each of the 229 scripts, decompiled and compiled again, is byte for byte the original
- [ ] #2 A script written by hand compiles, and our game runs it (put in an override folder, TASK-178.5)
- [ ] #3 More than 128 variables, a name not in the table, an unknown kind or line of speech: each gives an error naming the line and the word
- [ ] #4 Tests compile small scripts and check their bytes
<!-- AC:END -->
