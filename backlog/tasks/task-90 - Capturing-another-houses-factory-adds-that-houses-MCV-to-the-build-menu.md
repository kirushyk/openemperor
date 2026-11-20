---
id: TASK-90
title: Capturing another house's factory adds that house's MCV to the build menu
status: Done
assignee: []
created_date: '2026-11-20T23:36:00Z'
updated_date: '2026-11-20T23:47:00Z'
labels: []
dependencies: []
ordinal: 90000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User: started as Atreides, captured a Harkonnen factory with an engineer, but the Harkonnen MCV did not appear in the build menu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A captured factory of another house offers that house's MCV and vehicles it can make
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Menus list what each house the player holds a unit-making building or yard of makes, after the player's own, as Entry::MakeOf(role, house); production, progress, queue badges, cancelling and the building site go by house, so a captured Harkonnen factory makes a Harkonnen MCV, and the yard it deploys into Harkonnen buildings and walls. Icons are the making house's.
<!-- SECTION:FINAL_SUMMARY:END -->
