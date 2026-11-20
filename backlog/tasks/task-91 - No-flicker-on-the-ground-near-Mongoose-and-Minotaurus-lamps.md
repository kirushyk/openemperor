---
id: TASK-91
title: No flicker on the ground near Mongoose and Minotaurus lamps
status: In Progress
assignee: []
created_date: '2026-11-20T23:36:00Z'
updated_date: '2026-11-20T23:50:00Z'
labels: []
dependencies: []
ordinal: 91000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User: when Mongooses and Minotauruses walk with their lamps on, nearby ground triangles flicker.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The ground near a walking unit's lamp is lit steadily, without flicker
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-21: Not the lights: the ground is lit per pixel, and a walking Mongoose's or Minotaurus's lamp spot moves under 5 units a frame (disc check). Likely cause: the lamp beam is a see-through veil from the disc's model, drawn as a decal that reads depth but writes none, meeting the ground at a low angle, so it fights the ground for depth and the pattern shifts as the walk bobs it. shim/macos_metal.m now draws decals with a small slope-scaled depth bias toward the camera. Not seen yet: awaiting the user's look.
<!-- SECTION:NOTES:END -->
