---
id: TASK-135
title: Which things show no name under the cursor
status: To Do
assignee: []
created_date: '2026-11-22T01:16:00Z'
updated_date: '2026-11-22T01:54:00Z'
labels: []
dependencies: []
ordinal: 135000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Hanged figures and factory frigates show no tooltip when hovered. Check the rules for what says so and where else it applies, and follow it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The rules' flag for what shows no name is found and named
- [ ] #2 Everything it marks shows no tooltip, and everything else does
- [ ] #3 Tests cover it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
No rules key found. Names: gallows and house frigates are only in Text strings.txt (as are all units); E_Output_Text_Pickup.txt names barrels, trees, wrecks, the spotlight, the stunt frigate, Kynes's station. All have SLCT boxes. Ours: everything under the cursor shows its name. Asked the user which things show no name.
<!-- SECTION:NOTES:END -->
