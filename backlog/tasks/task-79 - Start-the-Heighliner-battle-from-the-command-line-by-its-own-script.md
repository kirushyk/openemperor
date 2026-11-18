---
id: TASK-79
title: Start the Heighliner battle from the command line, by its own script
status: Done
assignee: []
created_date: '2026-11-18T18:54:00Z'
updated_date: '2026-11-18T18:54:00Z'
labels: []
dependencies: []
ordinal: 79000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User: 'how to start with highliner mission'. The Heighliner battle ran with two MCVs, not its script on the disc, and --mission took only territory scripts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Heighliner battle runs its house's script from the disc
- [x] #2 --mission takes the Heighliner scripts' names, with or without their spaces, and --missions lists them
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
campaign::heighliner_script and HEIGHLINER_SCRIPTS; fight runs it for Mission::Heighliner; named_mission matches the names ignoring spaces and case; main checks a name by the battle it names. Checked by a temporary dump: the Atreides Heighliner opens with the script's Harkonnen troops, frigates and four snipers.
<!-- SECTION:FINAL_SUMMARY:END -->
