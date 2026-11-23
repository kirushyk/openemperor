---
id: TASK-181.1
title: The mission script language, written down
status: To Do
assignee: []
created_date: '2026-11-23T17:59:00Z'
labels: []
dependencies: []
parent_task_id: TASK-181
ordinal: 371000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Write docs/mission-script.md, the language as people write it, so that the decompiler and compiler work from one description.

What it needs to cover:
- **Lines.** A .tok keeps one statement a line, with comments and spacing dropped, and blank lines kept as empty lines so that line numbers match the source. The text form keeps that line for line: the original's parser reports errors by line ('Invalid line %d').
- **Declarations.** `int NAME`, `obj NAME` and `pos NAME`, for K162 to K164. A variable is a 0x81 token whose 7-bit number counts the declarations, so a script has at most 128. Names are lost in the .tok, so the decompiler makes them up (see TASK-181.2).
- **Statements.** `NAME = EXPR`, a call on its own, `if (EXPR)`, `else`, `endif`. Is `else` used in the 229, and is `if` ever nested? Find out from the scripts and from the interpreter's if/else/endif handling, whose messages are '%d If' at 0xf4710 and '%d Else (Ptr, %d) (Line, %d)' at 0xf4497.
- **Expressions.** Calls `Name(arg, …)`, with the arguments as plain commas and brackets between tokens. The operators ==, !=, >=, <=, >, <, &&, || and the binary + and -. Integers in decimal. TRUE and FALSE. Negatives as Neg(n), K143, which is how the scripts write them; whether `-4` is accepted and compiled to Neg(4) is to be decided. In what order operators bind, or whether they simply go left to right, comes from the interpreter's evaluation (the functions at 0xeec30 to 0xef4e0); write down which.
- **Kinds of unit and building** (0x82 tokens): by their Rules.txt names, written as `<ATConYard>`. A number has its Rules entry 111 places after it. The first 111 numbers are not known yet; write them as `<#57>`, so that every .tok can still be written as text.
- **Lines of speech** (0x83 tokens): by their ids in E_Output_Pickup.txt, written as `"ATstart1"`. A number is 502 more than the line's place among the MissionMessages sections. Numbers below 502 are not known yet; write them as `"#123"`.
- **Types.** int, obj and pos, and what each function takes and gives, from TASK-141 to TASK-149, and from the result types in docs/tok-format.md (1 a number, a truth or a side; 2 an object; 3 a position).
- **Comments.** `//` to the end of the line, dropped when compiling.
- **Characters.** Whatever the .tok keeps as plain characters (digits, commas, brackets, and any spaces that were left), so that a decompiled script compiles back to the same bytes.

Show it working with ATStart.tok's first 40 lines in the doc, as text beside their bytes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/mission-script.md describes every construct the 229 scripts use, with its tokens
- [ ] #2 How operators bind, and how if, else and endif nest, is written down from the original's code, with offsets
- [ ] #3 How unknown kinds and speech numbers are written is decided, so that every script can be written as text
- [ ] #4 docs/tok-format.md links to it
<!-- AC:END -->
