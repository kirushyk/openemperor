---
id: TASK-15
title: Skirmish selector and diplomacy screen show the houses' logos
status: Done
assignee: []
created_date: '2026-11-14T01:56:00Z'
updated_date: '2026-11-14T02:05:00Z'
labels: []
dependencies: []
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
In the skirmish selector and the diplomacy screen, show the houses' and mini-houses' logos from the disc instead of planets and shapes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The skirmish setup shows each house's logo from the disc where it now shows a planet
- [x] #2 The diplomacy screen shows each house's and mini-house's logo where it now shows a planet or shape
- [x] #3 Without the disc, the planets and shapes show as before
- [x] #4 Tests cover which picture shows with and without the disc
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The disc's 3DDATA has a logo per house (atlogo, hklogo, orlogo) and mini-house (frlogo, imlogo for the Sardaukar, ixlogo, tllogo, gulogo), 256 px with alpha. render/original.rs loads them as picture layers after the spice (original::logo(name)). The 2D vertex (OeVertex / platform::Vertex) gained u, v and a layer (-1 = none); the Metal overlay shader samples the picture array for a layer >= 0, and effects got their own fragment function. render.rs: draw_faction shows the house's logo in its ring where the planet was (skirmish faction button, opponents, diplomacy rows), draw_ally the mini-house's logo where its shape was; without the disc both fall back as before. Shaders compile offline; checked on the disc that the setup screen draws six logo quads on the right layers; not yet seen on screen.
<!-- SECTION:FINAL_SUMMARY:END -->
