---
id: TASK-31
title: Starport and ordering from it
status: Done
assignee: []
created_date: '2026-11-17T02:51:00Z'
updated_date: '2026-11-17T02:51:00Z'
labels: []
dependencies: []
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A starport building and the original's ordering: moving prices, stock, an order of up to six, a frigate countdown.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Starport is built after a hangar and the palace needs it
- [x] #2 Orders follow Rules' [General] starport values
- [x] #3 Tests cover it
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Role::Starport (1500, 100 power, needs a hangar, 9×7 or 9×8 for Ordos, disc models and pictures); the palace needs it. game::Market: offers per house from Rules' Starportable, prices repriced every 60 s within ±40%, stock +1 with 90% chance every 40 s, an order of up to six paid at once, delivered by units::deliver_at at the starport after 100 s, paid back if no starport stands. BuildTab::Starport from the wide button, with Order/Purchase/CancelOrder cells; a right click takes one off. Test bases have a starport. Frigate flight, AI use and the original's stock limits are logged.
<!-- SECTION:FINAL_SUMMARY:END -->
