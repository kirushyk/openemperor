---
id: TASK-122
title: Carryalls bringing harvesters stay the player's
status: Done
assignee: []
created_date: '2026-11-21T23:03:00Z'
updated_date: '2026-11-21T23:04:00Z'
labels: []
dependencies: []
ordinal: 122000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The carryall that brings the harvester of a new refinery or of a refinery's pad upgrade should be the player's own and stay on the battlefield.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each carryall that brings a harvester for a new refinery or a pad upgrade is its owner's and stays
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
A refinery's pad upgrade now delivers its harvester with stays = true, as a new refinery does; the carryall was already its owner's. Only the harvester given to a side short of them still comes by a carryall that leaves. The upgrade test now checks both carryalls stay.
<!-- SECTION:FINAL_SUMMARY:END -->
