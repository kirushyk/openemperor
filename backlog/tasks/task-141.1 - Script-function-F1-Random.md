---
id: TASK-141.1
title: Script function F1 Random
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 178000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf2288, 0x21 bytes; +0x400000 in memory): Random(n): gRandom (0x7c86e4) Next (0x51d400) modulo n (remainder of a signed divide), an int in 0..n-1 for positive n.  Used 55 times in 31 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's Random does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls Random and checks the effect
<!-- AC:END -->
