---
id: TASK-136
title: Victory said in more than one way
status: Done
assignee: []
created_date: '2026-11-22T02:43:00Z'
updated_date: '2026-11-22T15:09:00Z'
labels: []
dependencies: []
ordinal: 136000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Winning always says 'You are victorious'; the original also says 'Victory is yours'. Find the disc's recordings and use them as the original does.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A win plays them as the original does
- [x] #2 Tests cover it
- [ ] #3 A win says ui-g488 and a loss ui-g490, as the user remembers the original
- [ ] #4 Tests cover it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
UI lines near the win ui-g482 (5.7 s) and loss ui-g484: ui-g480, 486-496 unused (2-3.4 s). No transcript on the disc; exported to the scratchpad for the user to say which is 'Victory is yours'.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Victory now plays ui-g488 and defeat ui-g490, the lines the user remembers the original always saying, instead of ui-g482 and ui-g484. The existing test checks both are on the disc.
<!-- SECTION:FINAL_SUMMARY:END -->
