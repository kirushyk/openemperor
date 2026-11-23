---
id: TASK-178.3
title: Load the discs from .iso images
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-178
ordinal: 348000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Read ISO 9660 images directly, without mounting them. No new crate: Cargo.toml's dependencies stay empty, as the decisions doc has it.

What the English images are:
- The primary volume descriptor is at sector 16, a Joliet supplementary one at 17 (UCS-2 names, escape %/E, level 3) and the terminator at 18. Sectors are 2048 bytes.
- dune.iso has 318 003 sectors and is volume EMPEROR1. atreides.iso has 225 269 and is EMPEROR2.
- These are the user's own copies, in their home folder. Tests must not need them; an ignored test may use them when they are there.

What to read:
- The primary descriptor, whose volume id numbers the disc.
- The directory tree, by Joliet names when the image has them, since they keep mixed case ("Emperor Battle for Dune_EZ.exe"). Otherwise use the primary names, without their ';1'.
- Each directory record's extent, size and flags. Multi-extent and interleaved files need not be supported, but must be refused with a message.

A file on these discs lies in one run of sectors, so the image gives it as a byte range. An RFD or a bag inside the image is a range, and an entry inside it is a range within that range. FFmpeg's subfile URL on the .iso itself then plays the movies and music, as it does from a mounted disc now.

`--data` (TASK-178.2) takes one image, several, or a folder of them.

SafeDisc's files (secdrv.sys, drvmgt.dll, 00000001.TMP and so on) do not matter: our game never runs the original's exe.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `openemperor --data ~/dune.iso --data ~/atreides.iso` plays with nothing mounted: menus, maps, sounds, music, movies and speech
- [ ] #2 Names come from Joliet when the image has it, and from the primary directory otherwise
- [ ] #3 An image that is not ISO 9660, or needs something we do not support, gives a clear message and does not panic
- [ ] #4 Tests build a small image in memory, with primary and Joliet descriptors and nested folders, and read files from it by name and by range
- [ ] #5 No new dependency
<!-- AC:END -->
