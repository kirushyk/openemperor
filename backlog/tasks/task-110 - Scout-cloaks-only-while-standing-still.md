---
id: TASK-110
title: Scout cloaks only while standing still
status: Done
assignee: []
created_date: '2026-11-21T19:58:00Z'
updated_date: '2026-11-21T20:09:00Z'
labels: []
dependencies: []
ordinal: 110000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A scout should be in stealth only when not moving.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A scout that moves shows; standing still it goes invisible
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Role::stealthed_when_still (Scout, AtApc, the rules' StealthedWhenStill) keeps seen_moving at 1.2 s (StealthDelay 30) while moving, counting down once still; is_cloaked needs it at 0. The sniper's rules flag and StealthDelayAfterFiring are logged, not done.
<!-- SECTION:FINAL_SUMMARY:END -->
