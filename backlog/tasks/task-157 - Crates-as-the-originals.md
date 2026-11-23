---
id: TASK-157
title: Crates as the original's
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 157000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Crate kinds in the code (crate.cpp, path at file 0x1f4658, used at 0x9edeb): CashCrate (0x1f467c, used at 0x9f3f9), UnShroudCrate (0x1f4688, 0x9f392), StealthCrate (0x1f4698, 0x9f224) and StepOnCrate, a unit (0x1f46a8, 0x9f1bc), the rules' [CrateTypes] (0x20f41c) with CrateGiftObject (0x21216c), and the skirmish's Crates option (GameSetupCratesEdit 0x224380). We have money crates from wrecks only. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each crate kind does as the original's code does when picked up
- [ ] #2 A skirmish with crates on scatters them as the original does
<!-- AC:END -->
