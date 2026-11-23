---
id: TASK-178.4
title: 'Load the discs from raw images: BIN/CUE, MDF/MDS and NRG'
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-178
ordinal: 349000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Many copies of the discs are kept as raw images, not as .iso files.

BIN/CUE:
- The CUE names its BIN files and each track's mode.
- MODE1/2352 keeps each sector's 2048 bytes of data 16 bytes in. MODE2/2352 Form 1 keeps them 24 bytes in.
- MODE1/2048 is an ISO under another name.

MDF/MDS: an MDF is raw sectors of 2352 bytes, or 2448 with subchannel data. Its MDS gives the layout; without one, tell the layout from the sync pattern at the start of each sector.

NRG: the footer ("NER5" or "NERO" in the last 12 or 8 bytes) points to the track chunks, which give each track's offset and mode.

Behind each format, the ISO 9660 reader of TASK-178.3 reads 2048-byte sectors through a small mapping of sector to file and offset.

In a raw image a file is no longer one byte range, so FFmpeg's subfile URL cannot reach it. Movies and music in such an image go to FFmpeg through a read callback instead: an AVIOContext in shim/movie_ffmpeg.c, fed by our reader. The FFmpeg interface (oe_movie.h) changes for that, on every platform.

Audio tracks need not be read: these discs have none. No new crate: Cargo.toml's dependencies stay empty, as the decisions doc has it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A BIN/CUE of disc 1 in MODE1/2352 plays as well as its ISO does, movies and music included
- [ ] #2 MDF and NRG images are read, or refused with a message naming the format and what is missing
- [ ] #3 Tests build small raw images of each kind in memory and read a file from each
- [ ] #4 No new dependency
<!-- AC:END -->
