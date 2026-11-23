---
id: TASK-180.1
title: 'Sandbox battle: any map, any sides, a minor house to play'
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-180
ordinal: 362000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Set up a battle with more freedom than the skirmish screen gives.

The map: any of the 82, the campaign's included. Today --campaign-maps only lists them among the skirmish's.

The sides: as many as the map has start places, each of any house. That means the three great houses and the minor ones Rules.txt defines (Fremen, Sardaukar, Ix, Tleilaxu, Guild, which game.rs has as Ally), and the neutral sides the missions use.

The player can play a minor house, with its own units and buildings as Rules.txt gives them: the house's fields, and each item's prerequisites. A minor house has no construction yard in normal play, so it starts either with a set of units and buildings or with its main building (Ally::main_building) placed anywhere. Say which, and let the player choose.

For each side: cash, tech level, AI on or off and its difficulty, team, and start units.

For the battle: fog off, crates, worms on or off, game speed, and the random seed.

From the command line too, as the original's gDebugMapName did: `--map NAME --side atreides --side fremen:ai:hard …`.

Where a minor house lacks a voice, a sidebar icon or a portrait, use a stand-in, and list what was missing once in the log instead of failing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A battle starts on any map with any sides, from a screen and from the command line
- [ ] #2 The player can play each minor house and build what Rules.txt lets it build
- [ ] #3 Each per-side and per-battle setting above works
- [ ] #4 Missing art or speech for a minor house uses a stand-in and is listed, and nothing crashes
- [ ] #5 Tests set up sandbox battles and check their sides
<!-- AC:END -->
