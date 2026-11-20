---
id: TASK-89
title: 'Primary buildings: one of a kind makes the units'
status: Done
assignee: []
created_date: '2026-11-20T23:36:00Z'
updated_date: '2026-11-20T23:47:00Z'
labels: []
dependencies: []
ordinal: 89000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User: if two or more of the same kind of building exist, only one, the primary, produces units. The rules mark such buildings CanBePrimary = TRUE.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Of the player's buildings of a kind that can be primary, one is primary and makes and lets out that kind's units
- [x] #2 The player can make another one primary
- [x] #3 The primary is shown, and another takes over when it is lost
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
units.rs: Unit.primary; primary_maker picks, of an owner's buildings that can make a role (of a house if given), the one made primary or else the first put down; start_making and has_free_maker go to it only; make_primary; is_primary_of_several. game.rs: a second click within 0.4 s on one of the player's unit-making buildings makes it primary, with a chat line; render.rs writes Primary over the selected primary when there are two or more. A lost primary gives way to the next. The double click is how Westwood's Dune games set it; the disc's text does not say.
<!-- SECTION:FINAL_SUMMARY:END -->
