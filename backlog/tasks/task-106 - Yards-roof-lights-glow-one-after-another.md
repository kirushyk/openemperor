---
id: TASK-106
title: Yard's roof lights glow one after another
status: Done
assignee: []
created_date: '2026-11-21T19:56:00Z'
updated_date: '2026-11-21T20:06:00Z'
labels: []
dependencies: []
ordinal: 106000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The deployed construction yard's #Light marks all glow at once; the original lights them in turn.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The yard's numbered roof lights glow one at a time, in the order of their numbers
- [x] #2 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
A '#' light mark that stays still (one pose or none) and ends in a number takes turns with the others of its name (case aside): take_turns gives each its place by number, and glints shows it only in its 0.2 s turn. The yard's #Light01-09 chase; its two sweeping #light03/04 and its animated #Light10 glow all along. This applies to every model's still numbered lights (barracks, starports, palaces…); the 0.2 s is our choice.
<!-- SECTION:FINAL_SUMMARY:END -->
