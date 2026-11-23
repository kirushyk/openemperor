---
id: TASK-179.3
title: Unpack and pack the install cabinet
status: To Do
assignee: []
created_date: '2026-11-23T17:27:00Z'
labels: []
dependencies: []
parent_task_id: TASK-179
ordinal: 354000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
archive.rs reads Microsoft cabinets (MSCF): their head, any reserved room, their folders and file table, and folders kept STORED or as MSZIP. MSZIP blocks start "CK" and each may reach back into the one before.

Commands:
- `ls -l Game1.cab` lists its files with sizes, dates and attributes.
- `extract` takes all or some of its files out.
- `pack DIR -o x.cab` makes a single cabinet of one folder, MSZIP in 32 KB blocks (using TASK-179.2's deflate), or stored with --store.

Folders packed with LZX or Quantum are refused with a message. Game1.cab uses neither; DXSETUP's cabinets are DirectX's and are not needed.

Game1.cab is a single cabinet. A set (the previous and next flags) need only be listed. No new crate: Cargo.toml's dependencies stay empty, as the decisions doc has it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every file of Game1.cab is listed with its size and date, and extracts byte for byte as cabextract gives it
- [ ] #2 pack of the extracted Game1.cab lists the same files, sizes and dates, and our game reads it in place of the original
- [ ] #3 LZX, Quantum and sets of cabinets give a message, not a panic
- [ ] #4 Tests pack and read back small cabinets, stored and MSZIP
<!-- AC:END -->
