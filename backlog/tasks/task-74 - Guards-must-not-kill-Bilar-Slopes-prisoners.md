---
id: TASK-74
title: Guards must not kill Bilar Slopes' prisoners
status: Done
assignee: []
created_date: '2026-11-18T18:27:00Z'
updated_date: '2026-11-18T18:28:00Z'
labels: []
dependencies: []
ordinal: 74000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User: 'there were no prison camp!'. Launched with --mission ATP1D7FR, the five neutral civilians were shot by the Harkonnen guards within three seconds of the battle starting: can_attack counted nobody's units (owner NEUTRAL) as fair game, so guards, the AI and auto-fire all picked them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Units do not attack nobody's units (prisoners, barrels) of their own accord
- [x] #2 They still do when ordered to
- [x] #3 The prison camp's five prisoners are alive after ten seconds of --mission ATP1D7FR
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
can_attack now leaves out NEUTRAL; can_attack_when_told keeps them for orders (order_attack, the cursor, an ordered target). Checked by a temporary dump: all five alive after 10 s.
<!-- SECTION:FINAL_SUMMARY:END -->
