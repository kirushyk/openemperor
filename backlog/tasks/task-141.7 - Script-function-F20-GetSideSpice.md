---
id: TASK-141.7
title: Script function F20 GetSideSpice
status: Done
assignee: []
created_date: '2026-11-23T15:22:00Z'
updated_date: '2026-11-23T17:57:00Z'
labels: []
dependencies: []
parent_task_id: TASK-141
ordinal: 184000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
What the original does (case code at GAME.EXE file offset 0xf200d, 0xbe bytes; +0x400000 in memory): GetSideSpice(side): the same field as GetSideCash, Side+0x868 (spice and credits are one figure); summed over the human team in co-op.  Used 4 times in 2 of the 228 mission scripts. Argument passing, results and co-op handling are in docs/tok-format.md, 'How the interpreter runs a function'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The game does what the original's GetSideSpice does, as described here, in whatever form fits our code (it need not be one function)
- [x] #2 A test runs a script that calls GetSideSpice and checks the effect
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Script money as the original's Side+0x868: AddSideCash adds (it used to set), GetSideCash and GetSideSpice read the same figure, SetSideCash sets it only at update 0. The start mission now opens at 0 so its script's AddSideCash(player, 10000) is the 10 000 it starts with. Tested in mission.rs (a_sides_money_is_added_to_asked_of_as_cash_or_spice_and_set_only_at_the_start).
<!-- SECTION:FINAL_SUMMARY:END -->
