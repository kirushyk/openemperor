---
id: TASK-179.9
title: 'Sound bags: to WAV and back'
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-179
ordinal: 360000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The bags are GABA files (music.rs, sounds.rs):
- a 16-byte head: magic, version, track count and entry length;
- entries with a 32-byte name, then offset, size, rate, format flags and block size;
- sound kept as PCM, as IMA ADPCM at 4 bits a sample in blocks, or as MP3 (most of the Mentat's lines on the house discs).

The bags are INSTALL/DATA/MUSIC/music.bag, SFX/AUDIO.BAG and DIALOG/DIALOG.BAG, and each house disc's MUSIC.BAG and DIALOG.BAG.

Commands:
- `info x.bag` lists its tracks, with length in seconds, rate, channels and format, and the names sounds.txt and the Sfx tables use for them (which add the language's letter).
- `extract` writes WAV files: PCM as it is, ADPCM as IMA ADPCM WAV (format 0x11), or decoded with `--pcm`. MP3 is written as .mp3.
- `pack DIR -o x.bag` builds a bag from WAV and MP3 files, each kept in its own format. `--adpcm` encodes PCM to IMA ADPCM.

Today FFmpeg decodes these sounds. The tool must work without it for PCM and ADPCM, so write an IMA ADPCM decoder and encoder. First check that the bags' ADPCM is standard IMA and not a Westwood variant, and write down which. MP3 is passed through only. Unchanged tracks come back byte for byte. No new crate: Cargo.toml's dependencies stay empty, as the decisions doc has it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 info lists every track of every bag on the mounted discs
- [ ] #2 Every track extracts to a WAV or MP3 that a common player plays
- [ ] #3 extract then pack gives back the same bag byte for byte
- [ ] #4 A bag with a replaced track plays it in our game
- [ ] #5 IMA ADPCM decoder and encoder tested on their own, and which ADPCM the bags use is written in docs/files.md
<!-- AC:END -->
