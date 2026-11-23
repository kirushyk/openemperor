---
id: TASK-147
title: 'Script sides: neutral, coloured, given cash'
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 147000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Not carried out: SideNeutralTo F61 (65 uses in 21 scripts; case file 0xf16ac, 0x18f bytes), SetSideColor F150 (202 uses in 60 scripts; 0xf419c, 0x32) and SetSideCash F152 (0xf4212, 0x4c). GetSideColor F151 is unused. The colour numbers are the skirmish's (colour01.tga-colour09.tga, file 0x225620-0x2255a0). Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Sides made neutral to each other neither fight nor are attacked
- [ ] #2 A side the script colours is drawn in that colour
- [ ] #3 SetSideCash sets a side's money
<!-- AC:END -->
