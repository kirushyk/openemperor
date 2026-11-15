---
id: TASK-10
title: Show the player's invisible units as the original does
status: Done
assignee: []
created_date: '2026-11-14T01:28:00Z'
updated_date: '2026-11-14T04:51:00Z'
labels: []
dependencies: []
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Visualize invisibility of our units the way the original game does.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Units invisible to the enemy show to their own player the way the original draws them
- [x] #2 The enemy sees nothing of them, as before
- [x] #3 Tests cover which units are drawn that way
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
GAME.EXE loads textures\hackedstealth0..7.tga, a cyan, glassy 64px shimmer with alpha 71–200, into a table at 0x672314 and steps a frame counter through it. A stealthed batch is drawn with the current frame in place of its own picture, blended SRCALPHA/INVSRCALPHA, after two jittered DESTCOLOR passes. We now load the 8 frames with the disc's pictures (original::stealth picks a frame at 25 per second). render::cloak_model moves a cloaked unit's faces into see-through decals painted with that frame at their own UVs. Without the disc, the grey shimmer stays. The enemy still sees nothing (game test unchanged). The jitter passes are documented as not done. New render test covers both paths.
<!-- SECTION:FINAL_SUMMARY:END -->
