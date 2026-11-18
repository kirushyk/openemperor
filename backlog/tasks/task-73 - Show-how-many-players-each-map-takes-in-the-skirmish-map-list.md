---
id: TASK-73
title: Show how many players each map takes in the skirmish map list
status: Done
assignee: []
created_date: '2026-11-18T18:07:00Z'
updated_date: '2026-11-18T18:07:00Z'
labels: []
dependencies: []
ordinal: 73000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user asked: show number of players alongside maps.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each map in the open map dropdown shows how many players it takes beside its name
- [x] #2 The list's columns are wide enough for the longest name and its count
- [x] #3 The closed dropdown shows the chosen map's count where its name leaves room
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
players_note in game.rs gives 'N players'; ListLayout leaves room for the widest; render draws it right-aligned in each row and in the field.
<!-- SECTION:FINAL_SUMMARY:END -->
