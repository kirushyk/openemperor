---
id: TASK-101
title: Refinery pads slide out as the original's upgrades
status: Done
assignee: []
created_date: '2026-11-21T18:56:00Z'
updated_date: '2026-11-21T19:09:00Z'
labels: []
dependencies: []
ordinal: 101000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The disc's refinery model slides its two side pads out over frames 100-110 (Refinery Pad 1, 2) and back by 120; ours plays it in the Stationary loop, so pads come and go. Harvesters dock where our hand-made pads were, not the model's.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A side pad shows tucked in until its upgrade is done, slides out once, then stays out
- [x] #2 Pads' middles and angles are the disc model's, in the original's units, and harvesters dock on them
- [x] #3 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Pad parts (SmallPad0N) get their own frame: the start of their Refinery Pad N sequence until upgrade N is done, then as far as the refinery's popped (reset to 0 on the upgrade, out in 0.4 s), then its end. REFINERY_PADS holds each house's model's pad middles in world units, turned 0 and ±54 degrees; the first upgrade brings the right pad (SmallPad01), the second the left. The hand-drawn fallback converts them back and draws the middle pad's lamps; its made depth is 5.7356. Disc test checks tucked, sliding and out pads land on the docks for each house and facing.
<!-- SECTION:FINAL_SUMMARY:END -->
