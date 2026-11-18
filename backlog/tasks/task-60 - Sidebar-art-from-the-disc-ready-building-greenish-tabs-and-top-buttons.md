---
id: TASK-60
title: 'Sidebar art from the disc: ready building greenish, tabs and top buttons'
status: Done
assignee: []
created_date: '2026-11-18T15:48:00Z'
updated_date: '2026-11-18T16:00:00Z'
labels: []
dependencies: []
ordinal: 60000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A building ready to place should show greenish as in the original; the build tabs (infantry, vehicles, buildings) and the top buttons (sell, route, upgrade, starport) should use the disc's icons.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A finished building waiting to be placed shows its icon greenish
- [x] #2 The three build tabs are drawn with the disc's pictures
- [x] #3 Sell, route, upgrade and starport buttons are drawn with the disc's pictures
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Symbols read as frontend scenes (read_parts skips nested parts) and drawn along z with draw_icon, tinted down when greyed; ready buildings tinted READY_TINT. Frame checked for tabs and buttons; the green tint was not seen in a frame.
<!-- SECTION:FINAL_SUMMARY:END -->
