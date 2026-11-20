---
id: TASK-85
title: Veterancy as the original has it
status: Done
assignee: []
created_date: '2026-11-20T19:51:00Z'
updated_date: '2026-11-20T19:51:00Z'
labels: []
dependencies: []
ordinal: 85000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace the invented ranks (2/5/10 kills, +25% health, +20% damage, +10% speed, house-based healing) with each unit's own three VeterancyLevel blocks from Rules.txt, the kill Score from the rules, and the original's way of climbing levels.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A kill adds the dead unit's rules Score to an enemy killer's score
- [x] #2 Each unit reaches its own three levels at the rules' VeterancyLevel scores, and units the rules give none never rank
- [x] #3 Each level applies Health, Speed, ExtraDamage, ExtraArmour, ExtraRange, CanSelfRepair and StealthedWhenStill, keeping what lower levels set unless set again
- [x] #4 A new level keeps the share of health the unit had, as the original does
- [x] #5 Self-repair adds CanSelfRepair health each update up to the most it has
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Each unit's three levels come from its VeterancyLevel blocks in Rules.txt (46 kinds); a kill adds the rules' Score of the dead (0 where the rules give none, as walls). As the original: the score is compared against the next level's threshold each update and may climb several levels at once, the unit's type becomes the level's (so a level keeps what lower ones set), health keeps its share, and self-repair adds CanSelfRepair health per update. The rules' Elite flag has no effect found and is left out.
<!-- SECTION:FINAL_SUMMARY:END -->
