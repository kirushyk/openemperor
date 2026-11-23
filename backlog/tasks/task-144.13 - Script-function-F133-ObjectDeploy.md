---
id: TASK-144.13
title: Script function F133 ObjectDeploy
status: To Do
assignee: []
created_date: '2026-11-23T15:22:00Z'
labels: []
dependencies: []
parent_task_id: TASK-144
ordinal: 259000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf3aa2, 0x88 bytes; +0x400000 in memory): ObjectDeploy(obj): logs 'NonDeployableObject %d' if the object's virtual +0x98 is non-zero, 'AllReadyDeployed %d' if virtual +0xe8 says it is deployed; otherwise gives it order 0x22, deploy (virtual +0x228).  Used 14 times in 4 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The game does what the original's ObjectDeploy does, as described here, in whatever form fits our code (it need not be one function)
- [ ] #2 A test runs a script that calls ObjectDeploy and checks the effect
<!-- AC:END -->
