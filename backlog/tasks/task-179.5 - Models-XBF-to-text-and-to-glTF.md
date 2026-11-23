---
id: TASK-179.5
title: 'Models: XBF to text and to glTF'
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-179
ordinal: 356000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
xbf.rs reads XBF models: the pictures they are painted with; their objects, with meshes and transforms frame by frame; named sequences ("Move", "Fire 0") as frame ranges; parts whose picture slides (belts, tracks); and sounds at frames.

There are 1271 in 3DDATA0001 (units, buildings, explosions, crates, bullets, placement), 166 in UI0001 (front end, cursors, waypoints, in-game, sidebar), and one in each map's ground, test.xbf, with its start places.

Commands:
- `info x.xbf`: the tree of objects, with vertex and face counts, pictures, sequences and their frames, sounds, slides, and the dummies the game looks for (muzzles, smoke, where units come out).
- `convert x.xbf -o x.gltf`: glTF 2.0, as .gltf with its .bin and the pictures as PNG. The animation becomes keyframes, each sequence a named animation, and a house's paint (=paint.tga) is kept as its own material, so Blender opens it as the game shows it. `--house` paints it in a house's colour.

XAF, the original's effects (16 in 3DDATA, 2 in UI): `info` dumps one as text once TASK-174 knows its layout.

glTF back to XBF is not part of this task; it needs xbf.rs to write. Add a task for it once this is done. No new crate: Cargo.toml's dependencies stay empty, as the decisions doc has it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 info prints the parts above for every XBF on the discs without failing on any
- [ ] #2 Every unit's and building's model converts to glTF that Blender opens with its pictures and its sequences as animations
- [ ] #3 A map's test.xbf converts to a glTF of its ground
- [ ] #4 Tests convert a small model built in memory and check the glTF's JSON
<!-- AC:END -->
