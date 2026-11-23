---
id: TASK-179.4
title: 'Pictures: TGA and the maps'' wash to PNG and back'
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-179
ordinal: 355000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The pictures:
- TGA: 2460 textures in 3DDATA0001, the maps' thumbnails (thumb.tga, !05%thumb.tga), Bitmapfont.TGA, UI's ICONS\OR_CHAOS.TGA and the one in 3DDATA0002.
- xbf::tga reads uncompressed true-colour TGA only. Find what kinds the discs have: bits per pixel, RLE or not, origin, alpha. Read every one of them.
- A map's wash, in test.CPT: an 8-byte head with its width and height, then squares of 256 pixels, 8 to a row, in DXT3 blocks of 4 by 4 (xbf::wash).

Commands:
- `convert x.tga -o x.png`, and PNG back to TGA of the same kind, with the same header fields, so the game reads it as before. --to tga picks the kind for a new one.
- `convert test.CPT -o wash.png`, and a PNG back to a CPT, which needs a DXT3 encoder; a simple one is fine.

PNG is written with TASK-179.2's deflate and read with inflate.rs.

Decide whether `extract --png` converts on the way out; if so, it should still work as an override folder (TASK-178.5), converting back on read or written beside the original. No new crate: Cargo.toml's dependencies stay empty, as the decisions doc has it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every TGA on the discs converts to PNG and back to a TGA our game draws the same
- [ ] #2 Every map's wash converts to PNG and back to a CPT our game draws the same, up to DXT3's own loss
- [ ] #3 The kinds of TGA the discs have are written in docs/files.md
- [ ] #4 Tests cover each kind of TGA, PNG round trips, and DXT3 blocks
<!-- AC:END -->
