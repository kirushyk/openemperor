---
id: TASK-70
title: The original's scaffolding when a building goes up
status: To Do
assignee: []
created_date: '2026-11-18T16:25:00Z'
updated_date: '2026-11-23T14:33:00Z'
labels: []
dependencies: []
ordinal: 70000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Buildings deploying should show the original's scaffolding animation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A building being put down shows the disc's scaffolding as the original does
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
GAME.EXE (file offsets; +0x400000 for the address; map in docs/game-exe.md): the art list's construction model CArtIni::AddItemXafConstruction (0x1ee410, at 0x6f863; ArtIni key XafConstruction 0x1fccf4), the animation names Construct/Deconstruct (0x1ee134, 0x1ee152) and the building state Construction Anim (0x1f6254). See TASK-174.
<!-- SECTION:NOTES:END -->
