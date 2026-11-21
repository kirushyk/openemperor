---
id: TASK-99
title: Turn the camera with the trackpad
status: In Progress
assignee: []
created_date: '2026-11-21T17:20:00Z'
updated_date: '2026-11-21T17:36:00Z'
labels: []
dependencies: []
ordinal: 99000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rotating with two fingers on the trackpad should turn the camera.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A two-finger rotation on a trackpad turns the camera
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Twisting two fingers on a Mac trackpad was already passed to the camera (NSEventTypeRotate). A sideways two-finger sweep now turns it too, a third of a degree per fine delta, when it goes more sideways than up or down. Windows and Linux shims pass no trackpad gestures yet. Awaiting the user's try.
<!-- SECTION:NOTES:END -->
