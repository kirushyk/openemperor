---
id: TASK-178.1
title: A table of every kind of file and archive the game has
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-178
ordinal: 346000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Write docs/files.md, with one row per kind of file. Each row gives:
- the extension or name;
- where the files live: disc, folder, container;
- how many there are and their total size;
- what they hold;
- their layout in short, or a link to the doc that has it;
- what in GAME.EXE reads them, with offsets as docs/game-exe.md gives them;
- what in our code reads them, by module and function, or "not read";
- what the tool of TASK-179 can do with them: list, extract, convert to what, pack from what.

A second table lists the containers: each archive, its disc, where its index and its data are, how many entries it has and how many are packed, and the resource name the original's resource.cfg gives it (MODEL_DATA data\model, MAPS data\maps, UIDATA data\ui, 3DDATA, AI_DATA, MISSION_DATA, STRINGS, CAMPAIGN_DATA, SOUNDS, MOVIES, MOVIES1 to MOVIES4 and CD1 to CD4 as D:\, SAVED_GAMES data\saves, PLAYER_DETAILS, RECORD, LOGS).

What is there, counted on the English discs (dune.iso is EMPEROR1, 651 MB; atreides.iso is EMPEROR2; EMPEROR3 and EMPEROR4 are to be added once mounted):

Disc 1, INSTALL/Game1.cab (6.36 MB, MSZIP):
- GAME.EXE, binkw32.dll, Bitmapfont.TGA, resource.cfg, spice.ini (two numbers, 35 and 15), drvmgt.dll, secdrv.sys, 00000409.016 and .256;
- the 8 archive indexes data/*.RFH;
- the data of AI0001, CAMPAIGN0001, MISSIONS0001, MODEL0001, STRINGS0001 and UI0001;
- the loose tables data/Dialog/*.txt (8), data/Music/*.txt (5), data/Sfx/*.txt (8) and data/Sounds/sounds.txt;
- Westwood Online's pages in data/HTML (4 .htm, 8 .jpg, 1 .gif).

Disc 1, INSTALL/DATA:
- 3DDATA0001.RFD (68.5 MB), whose index is in the cabinet;
- 3DDATA0002.RFH and .RFD, holding one TGA;
- MAPS0001.RFD (242.7 MB);
- MOVIES/*.BIK (9 movies);
- MUSIC/music.bag, SFX/AUDIO.BAG and DIALOG/DIALOG.BAG.

The archives on disc 1:
- 3DDATA0001: 3749 entries, 3747 of them zlib. 2460 TGA (83.4 MB unpacked), 1271 XBF (73.6 MB), 16 XAF, dustbowl.ini and 1 TXT. Folders: Textures, Buildings, Units, Explosion, bullets, crates, Placement, Debris, spice, Health.
- AI0001: 218 TXT and 20 INI.
- CAMPAIGN0001: 19 TXT.
- MAPS0001: 533 entries. Each of the 82 maps has map.inf, test.CPF, test.CPT, test.lit, test.xbf and texture.dat. There are also 39 thumbnail TGAs and Baseinfo.ini.
- MISSIONS0001: 229 TOK and missionlist.ini.
- MODEL0001: Rules.txt and ArtIni.txt.
- STRINGS0001: 8 TXT and Subtitles\SubTitle.ini.
- UI0001: 166 XBF (FRONTEND, CURSORS, Waypoints, INGAME, SIDEBAR), 82 font MAPs, 33 TXT, 14 INI, 2 BAK, 2 XAF and 1 TGA.

Disc 2, the Atreides disc:
- MOVIES0001.RFH and .RFD: 20 BIK, 345.8 MB, all stored, none packed;
- MUSIC.BAG (53.3 MB) and DIALOG.BAG (59.5 MB);
- the installer's own files.

To settle while writing the table:
- What texture.dat holds. Each is as big as its map's test.CPT less 8 bytes.
- What the UI's .BAK files are: FRONTEND\OPTIONS.BAK and SIDEBAR\SIDEBAR.BAK, 1.18 MB together.
- The font .MAP files: 82 of 514 bytes, one per font page. The original's debug pane reads them with bitmapfont.tga ('Failed to find font map %s' at 0xa1d80; bitmapfont.tga at 0xa1eb0).
- XAF: what these effects hold. We read none of them yet (TASK-174).
- FRONTEND\FXEDITOR.INI: a config left over from Westwood's interface effects editor, with paths to their development drives (G:\Dev\Dune3\Art\UI_SCR~1). Check that nothing in GAME.EXE reads it.
- What 00000409.016, 00000409.256 and the *.TMP files beside the installer are.
- The discs' installer, DirectX, Westwood Online and SafeDisc files: list them, marked as not game data.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/files.md has a row for every extension on the discs and inside every container on them; a test, or the tool's `ls -R` counting kinds, shows none is left out
- [ ] #2 Each row says where the files are, how many, their size, what they hold, and what reads them in the original and in our game
- [ ] #3 A table of containers lists every archive, with its index, its data, its entries and its resource.cfg name
- [ ] #4 The open questions listed here are answered in it, or marked unknown with what was tried
- [ ] #5 docs/game-exe.md and docs/not-implemented.md link to it
<!-- AC:END -->
