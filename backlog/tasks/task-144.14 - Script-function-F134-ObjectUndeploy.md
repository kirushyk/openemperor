---
id: TASK-144.14
title: Script function F134 ObjectUndeploy
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-144
ordinal: 260000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3b2a, 0x5b bytes; +0x400000 in memory): ObjectUndeploy(obj): unused by the disc's scripts; the reverse of ObjectDeploy, logging 'NotCurrentlyDeployed %d' when it is not deployed.  No mission script on the disc uses it. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's ObjectUndeploy does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls ObjectUndeploy and checks the effect
<!-- AC:END -->
