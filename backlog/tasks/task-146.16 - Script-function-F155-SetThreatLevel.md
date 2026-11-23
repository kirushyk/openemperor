---
id: TASK-146.16
title: Script function F155 SetThreatLevel
status: To Do
assignee: []
created_date: '2026-11-23T15:23:00Z'
labels: []
dependencies: []
parent_task_id: TASK-146
ordinal: 274000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf42e0, 0x21 bytes; +0x400000 in memory): SetThreatLevel(kind, n): sets the threat value the AI gives a kind of unit or building to n (0x4290a0 -> CAiObjectKnowledge 0x43bf10, 'CAiObjectKnowledge: altering %s threat to %d'), the rules' AiThreat it weighs targets and dangers by. Every SideAI order first checks the side exists, then (if the AI manager, game 0x808d14 field +0x5c, says the side is not AI-run, 0x4285b0) hands it to the AI (0x4283f0(side, 1)), then calls CAiSide's mission command. Each mission command (CAiStrategyManager 0x44ebd0-0x44eea2) clears the side's running tactics (0x45c000), sets its mission mode (+0x34) and starts one tactic for all its units; mode numbers: 6 aggressive, 7 move, 8 attack object, 9 guard object, 10 exit map, 11 enter building, 12 headless chicken, 13 shuffle. Used 25 times in 14 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's SetThreatLevel does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls SetThreatLevel and checks the effect
<!-- AC:END -->
