---
id: TASK-144.15
title: Script function F136 ObjectRemove
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T18:16:00Z'
labels: []
dependencies: []
parent_task_id: TASK-144
ordinal: 261000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3b85, 0x2c bytes; +0x400000 in memory): ObjectRemove(obj): unused by the disc's scripts; gives the object order 0x2e (virtual +0x228), which takes it off the map.  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's ObjectRemove does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls ObjectRemove and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
ObjectRemove takes the thing off the map (Battle::remove, which ObjectSell used before). Tested in mission.rs.
<!-- SECTION:FINAL_SUMMARY:END -->
