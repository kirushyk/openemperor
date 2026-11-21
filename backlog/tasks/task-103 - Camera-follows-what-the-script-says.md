---
id: TASK-103
title: Camera follows what the script says
status: Done
assignee: []
created_date: '2026-11-21T18:56:00Z'
updated_date: '2026-11-21T19:15:00Z'
labels: []
dependencies: []
ordinal: 103000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The tutorial's opening calls F107 on its harvester before the scene locks the screen, and F108 later.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 F107(thing) keeps the camera on the thing, or on the carryall holding it
- [x] #2 F108() stops it
- [x] #3 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
F107(thing) is Shot::Follow, setting the game's following; F108() Shot::StopFollowing clears it. follow_with_camera now also finds a vehicle among those carryalls hold. Tests: the script's shots, and the camera on a harvester brought by carryall until told to stop.
<!-- SECTION:FINAL_SUMMARY:END -->
