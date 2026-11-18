---
id: TASK-56
title: Infantry cheer on victory
status: Done
assignee: []
created_date: '2026-11-18T14:39:00Z'
updated_date: '2026-11-18T14:52:00Z'
labels: []
dependencies: []
ordinal: 56000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Infantry should play a cheering animation when their side wins.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 On victory the winning side's infantry play the model's cheer animation
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
units::cheer, called from Game::decide, stops the winners' infantry and sets cheering; original::frame plays the model's "Win" while a cheering soldier stands. Tests in game.rs and original.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
