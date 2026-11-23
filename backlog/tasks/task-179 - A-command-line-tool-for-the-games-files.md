---
id: TASK-179
title: A command-line tool for the game's files
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
ordinal: 351000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A second program beside the game, `oe-files`, for listing, unpacking, packing and converting everything on the discs. It reads through the layer of TASK-178.2 and takes the same --data options as the game.

It must be a binary of its own. The game's is built for the Windows GUI subsystem (the first line of src/main.rs), which loses console output. The readers both need (archive.rs, inflate.rs, xbf.rs, mission.rs's script reader, the resource layer) go behind a library target, src/lib.rs, or are shared in whatever way keeps the change small. No new crate: Cargo.toml's dependencies stay empty, as the decisions doc has it. Its argument parsing, PNG, WAV and deflate are all our own.

Paths are named as TASK-178.2 names them: a disc path, a container entry, or a resource name. Each subtask adds the commands for one kind of file. The group's own subtask is the frame:
- `sources`: what was found (discs, images, folders, overrides) and what each is;
- `ls [-l] [-R] [PATH]`: browse it all like one file system, a container showing as a folder (Game1.cab/, MAPS0001/), with sizes, whether packed, and kind;
- `find PATTERN`: a glob across every source and container;
- `cat PATH`: an entry's bytes to stdout, unpacked;
- `info PATH`: what the file is and what is in it, by kind;
- `extract PATH… -o DIR`: files or whole containers to a folder, paths kept, in the override layout of TASK-178.5;
- `pack` and `convert`, by kind, in the subtasks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All its subtasks are done
- [ ] #2 `oe-files --help` and `oe-files <command> --help` explain every command
<!-- AC:END -->
