---
id: TASK-155
title: Sandstorms
status: To Do
assignee: []
created_date: '2026-11-23T14:32:00Z'
labels: []
dependencies: []
ordinal: 155000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original's storms: sandstormobject.cpp (path at file 0x21f36c, used at 0x12be08), the rules' [General] StormMinDistance ... StormMaxLife (file 0x21d5e4-0x21de7c, 12 keys), StormDamage (0x219124) and the SandStorm kind (0x21905c), the campaign's CampaignStorms (0x1f1f5c) with the territories' PhaseStormPercentage (0x1fcda8) and its Stormwarning briefing (0x1f2240), and the StormUnit emitter ('Expected to find a storm object', 0x2212ec, used at 0x144302). We have the tornado's look only. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Storms come and go on the battlefield as the rules' figures say, moving, turning and hurting what they cross
- [ ] #2 A territory's storm chance brings storms into its battle, with the warning
<!-- AC:END -->
