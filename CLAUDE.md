# OpenEmperor

## Rules for every change

- Everything is idiomatic Rust.
- Changes are minimal: touch only what the work needs.
- Code goes at the proper level of logic and in the proper location: simulation in
  `units.rs` and `weapons.rs`, game flow and input in `game.rs`, layout in `hud.rs`,
  drawing in `render.rs`, the OS boundary in `platform.rs` and `shim/`.

## Formatting

All Rust code is formatted with `cargo fmt`, using rustfmt's default settings. Git hooks in
`.githooks/` check this before a commit, merge, rebase or push. After cloning, turn them on once:

    git config core.hooksPath .githooks
