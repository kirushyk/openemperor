---
id: TASK-178.2
title: One place to find the game's files, wherever they are
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-178
ordinal: 347000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Today discs.rs looks under /Volumes/EMPEROR1 to EMPEROR4, and only on macOS. Each module joins its own constants onto the disc path (INSTALL/Game1.cab, \data\MAPS0001.RFH, INSTALL/DATA/MAPS0001.RFD and so on), and maps.rs, sounds.rs and campaign.rs each read Game1.cab again. video.rs and music.rs hand FFmpeg a `subfile,,start,…,end,…:` URL into a file. On Windows and Linux nothing is found.

Make one resource layer, in archive.rs and discs.rs or a module of its own if that is the proper place.

It knows the sources. Each is a disc, by its number, whatever form the disc takes:
- a mounted disc;
- a folder the disc was copied to;
- a disc image (TASK-178.3, TASK-178.4);
- an installed copy of the original, with its data folders as resource.cfg lays them out.

It tells discs apart by their volume name (EMPEROR1 to EMPEROR4) or by what they have: disc 1 has INSTALL\, and a house disc has MOVIES0001.RFH. A house disc's house is known from what it holds, such as its movies' names, not from its number.

It finds the sources:
- on macOS, under /Volumes, as now;
- on Windows, on every CD drive (resource.cfg's CD1 to CD4 assume D:\);
- on Linux, under /media/$USER, /run/media/$USER and /mnt;
- given with `--data <path>`, which can be repeated: a folder, a disc image, or a folder holding disc images;
- given in OPENEMPEROR_DATA, a list like PATH;
- given in the options file, once TASK-163 has one.

It gives any file by one name:
- a path as on the disc, such as INSTALL/DATA/MOVIES/LEGALS.BIK;
- an entry inside a container, such as `MAPS0001/#A1 JF Caladan Capital LOD2/test.xbf` or `Game1.cab/data/Sfx/GeneralSFX.txt`;
- a path under one of the original's resource names from resource.cfg, such as MODEL_DATA/Rules.txt.

Names match in any case, with \ and / alike. A file comes back as bytes. A file kept whole, not packed, can also come back as a byte range (source file, offset, length), so FFmpeg still streams movies and bags without copying them.

It lists everything, sources, containers and entries, for the tool (TASK-179) and the resource browser (TASK-180.5).

The original does this through its CResourceManager:
- resource.cfg, in the cabinet, names each kind's folder. It is read at 0x5560 (0x100 bytes), which fails with 'Failed to find second line in RESOURCE.CFG'.
- It opens libraries as <name><NNNN>.RFH and .RFD ('%s%04lu%s', at 0x2f80, 0x3ab0 and 0x3d4b).
- It streams an entry only if it is neither packed nor cached ('Cannot stream file because %s … is compressed', 0x3d4b).

Those are its behaviour, not a structure to copy. Offsets are into GAME.EXE as Game1.cab holds it (2 449 408 bytes); add 0x400000 for the address in memory (docs/game-exe.md).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every module that reads game data gets it through the one layer; none joins disc paths of its own
- [ ] #2 The game finds its discs mounted as usual on macOS, Windows and Linux, and in any folder or image given with --data or OPENEMPEROR_DATA
- [ ] #3 Discs are told apart by their volume name or their contents, not by where they are mounted
- [ ] #4 Movies, music and speech still stream from inside archives and bags without being copied out
- [ ] #5 Game1.cab is read once a run
- [ ] #6 Tests find files by disc path, by container entry and by resource name, in any case and with either slash
<!-- AC:END -->
