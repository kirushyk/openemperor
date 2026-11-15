---
id: TASK-12
title: Infantry die with the original's death animations
status: Done
assignee: []
created_date: '2026-11-14T01:28:00Z'
updated_date: '2026-11-14T04:47:00Z'
labels: []
dependencies: []
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace our ragdoll (commented out) with the original's death animations.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Our ragdoll for dying infantry is commented out, not deleted
- [x] #2 A dying soldier plays its model's death run that fits how it died, and lies as the run ends until its body goes
- [x] #3 Tests cover which run is picked and that it is played to its end
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Dead infantry leave a weapons::Body instead of a ragdoll. drop_body's ragdoll code, its constants and its tests are commented out, not deleted. Each unit records a units::Wound: set by the weapon that hit it (flame burns, gas and cisterns gas, missiles and shells blow up, others shoot), by gas clouds and poison, and by being run over. A fresh blast around the soldier makes it blown up. render/original.rs draw_body plays the matching run from the disc model ('Shot 1/2', 'Blow Up 1/2', 'Burnt 1', 'Gassed 1', 'Run Over 1', or 'Deployed Death 1/2' when dug in, falling back to 'Shot 1'). It holds the last frame, then sinks over the last second of the body's 10. Without the disc, no body is drawn. Tests cover the wounds, run choice, playing to the end, and the body's life.
<!-- SECTION:FINAL_SUMMARY:END -->
