<!-- looper:begin rust v3 -->
# Rust

The rules of this project's Rust code, on top of `AGENTS.md`.

- **A debug cargo command never takes `-p`.** In a workspace, `-p` resolves features for that one
  package and builds a second copy of every dependency. Narrow what runs with `--test <name>`, `--lib`
  or a test name filter.
- **`looper verify` runs `cargo fmt`, then clippy with `-D warnings`, then the tests a change reaches.**
  A warning fails the gate, so fix it rather than allow it.
- **A stage other than the gate stage checks with a build alone:** `cargo build`, then
  `cargo clippy --all-targets -- -D warnings`. Clippy compiles the tests too, so a test that no longer
  builds shows there.
- **The toolchain is pinned in `rust-toolchain.toml`.** Bump it in a commit of its own.
- **By default, the lints live in the workspace `Cargo.toml`,** and each crate takes them with
  `[lints] workspace = true`. Unsafe code is forbidden. A project may change this in its own part of
  `AGENTS.md`.
- **A crate's module doc is its head comment,** in the shape `docs/agent/conventions.md` gives.
