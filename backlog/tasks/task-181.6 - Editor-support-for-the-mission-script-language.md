---
id: TASK-181.6
title: Editor support for the mission script language
status: To Do
assignee: []
created_date: '2026-11-23T17:59:00Z'
labels: []
dependencies: []
parent_task_id: TASK-181
ordinal: 376000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Make writing missions comfortable in VS Code, and in any editor that takes TextMate grammars:
- a grammar that colours keywords, functions, types, kinds, speech, numbers and comments;
- completion of functions, with each one's arguments and a line saying what it does, from TASK-141 to TASK-149; of kinds, from Rules.txt; and of speech ids, from E_Output_Pickup.txt, with their words;
- the compiler's errors and warnings (TASK-181.4) shown in the editor, through a problem matcher on `oe-files script check`, its output as file:line:column: message.

Keep it in the repository, in its own folder such as editors/vscode. It is built from data the tool prints (`oe-files script functions --json`, with kinds and speech from the discs), so nothing of the original's data is committed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The grammar colours every one of the 229 decompiled scripts without a gap
- [ ] #2 Completion offers functions with their arguments, and kinds and speech from the discs
- [ ] #3 Compiler errors show at their line in VS Code
- [ ] #4 No data of the original's is committed
<!-- AC:END -->
