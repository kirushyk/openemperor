---
id: TASK-7
title: Damaged buildings look damaged, as in the original
status: Done
assignee: []
created_date: '2026-11-14T01:28:00Z'
updated_date: '2026-11-14T01:59:00Z'
labels: []
dependencies: []
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
If the original shows buildings damaged, copy how.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Find out how the original shows a damaged building: model variants, fire and smoke, or picture changes
- [x] #2 Ours show the same at the same health, from the disc's data
- [x] #3 Tests cover what shows at which health
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The original shows damage with its own models: each building and tower has _H1 and _H2 models beside its whole _H0, made with _D_ pictures (cracked, burnt), plus _H3 (Explode) and _HC (Construct). render/original.rs reads H1 and H2 for all 29 buildings and towers we draw (259 picture layers in all) and shows H1 below half health and H2 below a quarter. The disc gives no thresholds; these are Westwood's usual yellow and red. DamageEffect fire in Rules.txt is only for vehicles, so not part of this.
<!-- SECTION:FINAL_SUMMARY:END -->
