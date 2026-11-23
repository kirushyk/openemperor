---
id: TASK-180.5
title: A browser of the game's files, in the game
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-180
ordinal: 366000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A screen in geek mode's menu for walking through everything the resource layer (TASK-178.2) finds. It shows a tree of sources, containers and entries, with search, and a preview for each kind of file:
- pictures: TGA and the maps' wash;
- models: an XBF turning in light, playing its sequences, with its objects shown or hidden, its pictures, and a house's colour;
- the original's effects (XAF), once TASK-174 reads them;
- maps: the layers of TASK-179.6, and a flight over the ground;
- sounds and music: playing them, under the tables' names, with which sections use them;
- movies: playing them, with their subtitles;
- text and INI files;
- scripts, as TASK-179.7's text;
- Rules.txt entries: a unit's fields, linked to its model, weapons and sounds;
- the strings, by id.

What is selected can be exported, through the tool's code (TASK-179).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every file the layer lists can be reached from the tree or by search
- [ ] #2 Each kind above previews in the game
- [ ] #3 Export writes the same files as `oe-files extract` and convert would
<!-- AC:END -->
