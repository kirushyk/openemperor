---
id: TASK-77
title: Destroyed wrecks give money crates
status: Done
assignee: []
created_date: '2026-11-18T18:28:00Z'
updated_date: '2026-11-20T23:55:00Z'
labels: []
dependencies: []
ordinal: 77000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User: 'wreck should on destruction give money crates'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A wreck destroyed leaves money crates as the rules say
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Role::crate_when_gone reads the rules' ObjectTypeWhenGone for what we have crates of: INBuggyWreck, INHarvesterWreck, INMedicalWreck and INSandCrawlerWreck leave a MoneyCrate (CASH2000), one each as their NumInfantryWhenGone = 1 says; IXTransport a ProjectorCrate (an infiltrator). settle_casualties puts it where the thing stood, lying until found as a map's own crates do. Not done: AKINNoddingDonkey's CashCrate, TLTransport's LeechCrate (no leech yet), AKINDrKynes's Fedaykin and the Fremen's worm sign.
<!-- SECTION:FINAL_SUMMARY:END -->
