---
id: TASK-159
title: 'The original''s AI: tactics and scripts'
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 159000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original attacks and defends with scripts read from AI_scripts.txt (name at file 0x1c4e58; 'Loaded %d scripts' 0x1e8aa4, used at 0x457b8) and objectsets.txt (0x1e759c, read at 0x3c59c): chunks DEFINES, STEP, MONITOR, STRATEGY, TAUNT, OBJECTSET, STAGING, TARGET, TEAM, DESCRIPTION (0x1e8b08-0x1e8c24) and keywords (0x1e8778-0x1e8a7c: frequency, priority, teamtype, minunits, maxunits, targettype, staging, route, encounter, formation...). CAiTacticsManager picks proactive and reactive scripts (0x1ea4a4 used at 0x4e292, 0x1ea420 at 0x4e19f), teams form (Circle, Protect, Wedge, MultipleLines 0x1ecb08-0x1ecb20) and fight; built-in tactics defend the yard and harvesters, scout, lift by air and defend against aircraft. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The computer's attacks and defences come from AI_scripts.txt as the original's tactics manager picks them
- [ ] #2 Teams form, stage and attack as their scripts say
- [ ] #3 Tests run a script's team from forming to its battle
<!-- AC:END -->
