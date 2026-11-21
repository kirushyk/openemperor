---
id: TASK-96
title: Kindjal speaks when it deploys
status: Done
assignee: []
created_date: '2026-11-21T05:45:00Z'
updated_date: '2026-11-21T05:57:00Z'
labels: []
dependencies: []
ordinal: 96000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A Kindjal is silent when deployed; the original says something.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Deploying a Kindjal plays the original's response
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The disc's sound tables give units that deploy a line for it, in their Special0 section, and one for packing up, in Special1: a Kindjal's 00-UX00 and 00-UX02, an MCV's, a mortar's, a projector's. The player's units that have just begun to deploy or pack up, by the D key or their cursor, now say it. Tested.
<!-- SECTION:FINAL_SUMMARY:END -->
