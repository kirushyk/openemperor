---
id: TASK-168
title: Tooltips and sidebar hints
status: To Do
assignee: []
created_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 168000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Tooltips\%s.ini (file 0x1f692c, read at 0xb7614), SidebarHints (0x1f6954, at 0xb7d60), ObjectTips, TerrainTips (0x1f6c9c, at 0xc0237) and SidebarTips (0x1f7784): what the cursor is over is named, and the sidebar's icons explained. Option Tooltips turns them on and off. TASK-135 is which things show no name. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory. The map is docs/game-exe.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Hovering over a unit, building or ground shows the original's tooltip
- [ ] #2 Hovering over a sidebar icon shows its hint
<!-- AC:END -->
