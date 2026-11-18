---
id: TASK-39
title: 'Atreides first mission: its script''s bases, reinforcements and Mentat lines'
status: Done
assignee: []
created_date: '2026-11-18T04:07:00Z'
updated_date: '2026-11-18T04:27:00Z'
labels: []
dependencies: []
ordinal: 39000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Run enough of ATStart.tok for the first Atreides battle: the bases it builds for both sides, the reinforcements it brings in on time, and the Mentat's lines it plays.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The first Atreides battle starts with the script's bases
- [x] #2 The script's reinforcements arrive when it says
- [x] #3 The Mentat's lines play when the script says
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
ATStart.tok runs through mission::Script: bases, reinforcements and waves, Mentat lines via E_Output_Pickup + sounds.txt off the house disc, victory when palace, outpost and hangar fall. Update rate 0.1 s is a guess; unknown functions do nothing.
<!-- SECTION:NOTES:END -->
