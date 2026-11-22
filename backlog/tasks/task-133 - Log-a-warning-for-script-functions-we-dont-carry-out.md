---
id: TASK-133
title: Log a warning for script functions we don't carry out
status: To Do
assignee: []
created_date: '2026-11-22T01:13:00Z'
labels: []
dependencies: []
ordinal: 133000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
When a mission script calls a function we have not implemented, log a warning naming it, so nothing is skipped silently.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A script calling a function we don't carry out logs a warning naming it, once per function
- [ ] #2 Tests cover it
<!-- AC:END -->
