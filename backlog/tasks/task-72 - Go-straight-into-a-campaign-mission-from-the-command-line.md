---
id: TASK-72
title: Go straight into a campaign mission from the command line
status: Done
assignee: []
created_date: '2026-11-18T18:04:00Z'
updated_date: '2026-11-18T18:04:00Z'
labels: []
dependencies: []
ordinal: 72000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user asked for a command-line way to launch a territory's battle such as Bilar Slopes' walled camp (ATP1M7FR), and to list the available ones.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 --mission NAME starts the territory battle the disc's script of that name gives, in a campaign set up at that phase
- [x] #2 --missions lists every such battle with its house, phase and title, and quits
- [x] #3 An unknown name or a missing one is an error that says so
- [x] #4 Tests cover the name parsing, the campaign set-up and the options
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added campaign::named_mission, mission_names and Campaign::at, Game::play_mission, and Options parsing in main.rs with --mission and --missions. mission_script now ignores the Fail/Win scripts, whatever their order on the disc.
<!-- SECTION:FINAL_SUMMARY:END -->
