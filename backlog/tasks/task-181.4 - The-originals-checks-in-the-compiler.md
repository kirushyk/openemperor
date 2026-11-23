---
id: TASK-181.4
title: The original's checks, in the compiler
status: To Do
assignee: []
created_date: '2026-11-23T17:59:00Z'
labels: []
dependencies: []
parent_task_id: TASK-181
ordinal: 374000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Make the compiler refuse what the original's MissionParser refused, with the same reasons, given as file:line:column and what was wrong:
- assignment of a different type: ' Invalid assignment (=) - different types ' (0xeec30);
- comparisons: == and != of different types (0xeed10, 0xeedb0); >=, <=, > and < of a type not int, or of different types (0xeee50, 0xeef40, 0xef030 and 0xef089, 0xef120);
- && and || (0xef210, 0xef303), and + and - (0xef3f0, 0xef4e0), on operands they cannot take;
- if and endif not balanced: 'Incorrect if/endif count' (0xf5859);
- a call with the wrong arguments: ' Invalid Parameter List ' (0xefa6f) and ' Invalid Parameter ' (0xf0e38). The argument count and type of each function come from TASK-141 to TASK-149, and from the 0x5c-byte records of the names table, if their seven numbers after the name are argument types (AirStrike's are 4, 4, 4, 4, 4, 4, 4); find out;
- a script point out of range: 'Invalid script point.' (0xf06cd);
- a kind that is not a building or unit: 'Object Type is not a building or unit type' (0xedf80).

Read each check's code and write down exactly what it refuses, in docs/mission-script.md. Some checks the original only makes as the script runs; say which, and have the compiler make them where it can tell beforehand.

Warnings, which do not stop the compile:
- variables declared and never used;
- variables used before they are set;
- functions our game does not carry out yet, by name, with their task numbers (TASK-141 to TASK-149), so a script author knows what will not work. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory (docs/game-exe.md).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each check above, read from the original's code, is described in docs/mission-script.md with its offset
- [ ] #2 The compiler refuses what each check refuses, with its message, file, line and column
- [ ] #3 None of the 229 original scripts gives an error
- [ ] #4 Tests give one wrong script for each check and expect its message
<!-- AC:END -->
