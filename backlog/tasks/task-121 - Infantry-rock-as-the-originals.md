---
id: TASK-121
title: Infantry rock as the original's
status: Done
assignee: []
created_date: '2026-11-21T22:19:00Z'
updated_date: '2026-11-21T22:24:00Z'
labels: []
dependencies: []
ordinal: 121000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Infantry rock (InfRock) is ground on low or high ground where infantry stand in a defensive position: they take less damage, see and reach farther, and vehicles cannot crush them there.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Infantry on infantry rock take the rules' share of damage (Armour ..., 50, InfRock)
- [x] #2 Units on infantry rock see as far as the second figure of their ViewRange
- [x] #3 Infantry on infantry rock reach InfRockRangeBonus tiles farther
- [x] #4 Vehicles cannot crush infantry on infantry rock
- [x] #5 Only units whose Terrain lists InfRock go onto it
- [x] #6 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Unit::on_inf_rock is set each advance for soldiers on the ground on Ground::InfRock. There harm_taken halves damage, range adds InfRockRangeBonus 2 tiles (not the contaminator), and can_run_over/can_squash spare them. Role::rock_view_range holds the machines' second ViewRange figure; Fog::see_rock spreads each machine's rock sight over infantry-rock points only, after update. Only infantry could already cross it. Tests: soldier on rock, a machine's rock sight, fog over rock.
<!-- SECTION:FINAL_SUMMARY:END -->
