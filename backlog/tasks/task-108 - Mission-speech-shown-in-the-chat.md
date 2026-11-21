---
id: TASK-108
title: Mission speech shown in the chat
status: Done
assignee: []
created_date: '2026-11-21T19:56:00Z'
updated_date: '2026-11-21T20:13:00Z'
labels: []
dependencies: []
ordinal: 108000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The original writes each spoken mission line in the chat as well.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each line of mission speech that plays shows its text in the chat
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
sounds reads each MissionMessages entry's text (between its braces, spaces collapsed) beside its recording; mission_text(n) gives it. Game::say posts it to the chat wrapped (font::wrapped, moved from render) to hud::chat_width, left of the build tabs. Disc test: the tutorial's and the Mentat's lines.
<!-- SECTION:FINAL_SUMMARY:END -->
