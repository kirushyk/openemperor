---
id: TASK-179.8
title: 'Movies: list, extract and convert'
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-179
ordinal: 359000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The movies are Bink 1 (BIK): 9 loose in INSTALL/DATA/MOVIES, and 20 on each house disc in MOVIES0001.RFD, all stored. Their subtitles are in STRINGS0001's Subtitles\SubTitle.ini (TASK-176).

Commands:
- `info x.bik` gives, from its header, the frames, the rate, the size, and its audio tracks with their rate and channels.
- `extract` copies them out through the frame.
- `convert x.bik -o x.mkv` (or .mp4) goes through FFmpeg, which the game already loads (shim/movie_ffmpeg.c). Where FFmpeg is missing, it says so; the rest of the tool needs no FFmpeg.
- `convert --subtitles x.bik -o x.srt` writes its subtitles out.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 info reads every movie's header on the mounted discs
- [ ] #2 Every movie extracts byte for byte, loose or from an archive or image
- [ ] #3 convert writes a file a common player plays, with sound, when FFmpeg is there, and a clear message when it is not
- [ ] #4 Subtitles come out as .srt for the movies that have them
<!-- AC:END -->
