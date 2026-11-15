# OpenEmperor

## Rules for every change

- Everything is idiomatic Rust.
- Changes are minimal: touch only what the work needs.
- Code goes at the proper level of logic and in the proper location: simulation in
  `units.rs` and `weapons.rs`, game flow and input in `game.rs`, layout in `hud.rs`,
  drawing in `render.rs`, the OS boundary in `platform.rs` and `shim/`.

## Backlog

Tasks are kept with [Backlog.md](https://github.com/MrLesk/Backlog.md) in `backlog/`, through
its `backlog` command (`backlog instructions overview` explains the workflow).

- Every task goes into the backlog before work on it starts, one-shot tasks too:
  `backlog task create "Title" -d "…" --ac "…"`.
- Every task has acceptance criteria: what must be true for it to count as done.
- Set a task to "In Progress" when work on it starts. Once all its criteria hold and its
  tests pass, tick them (`--check-ac`), add a final summary and set it to "Done".
- When idle, with nothing asked, take a task from "To Do"
  (`backlog task list -s "To Do" --plain`), solve it, and finish it as above.

## Formatting

All Rust code is formatted with `cargo fmt`, using rustfmt's default settings. Git hooks in
`.githooks/` check this before a commit, merge, rebase or push. After cloning, turn them on once:

    git config core.hooksPath .githooks
