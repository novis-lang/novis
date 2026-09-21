# Handoff

## State

Goal `core-db-connection-and-2-more` (milestone `dossier`) is met: its three groups —
`Core\Db\Connection`, `Core\Db\Plan` and `Core\Db\Plan\Step` — owe nothing, and every example and
attack under them passes. `Core\Db\Plan\Step::reason` and `Core\Db\Plan\Step::isRefused` landed this
session, each with a description, three examples, an attack, a bench, a `.nvst` case and a Rust
`#[test]`. One hostile file under `Core\Db\Connection::query` stays marked `known-gap` and is
recorded in `crates/nvs-stdlib/src/db/mod.rs`. `python tools/owners.py --closes` and `python
tools/playbook.py --closes` name no gap for this goal.

Nothing is blocked. One finding stays recorded rather than fixed: `Core\Db\Queryable` is not a type
a program can write, so no function can take a connection and a transaction alike —
`crates/nvs-stdlib/src/db/mod.rs` `# Known gaps` item 2.

## Next group

The chain's next goal is `core-db-row`, and `goal-switch.py` installs that goal's own generated
handoff over this file. If the switch has not happened, these are its first three slices — one file
set: `crates/nvs-stdlib/src/db/row.rs`, `nvs.toml`, the `core/Db-Row/` directories in the three
proof trees, and `tests/conformance/core/`. `rule:testing/feature-proofs` is what they owe, and one
slice is one feature with all of them.

- [ ] **`Core\Db\Row::bool`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1401`
- [ ] **`Core\Db\Row::bytes`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1365`
- [ ] **`Core\Db\Row::date`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1428`

## Backlog

- `Core\Db\Queryable` is unwritable in a program — `crates/nvs-stdlib/src/db/mod.rs` `# Known gaps`
  item 2, and the only finding this goal recorded rather than fixed.
- A `Core` member whose value is computed in another crate needs that crate's producing file in
  `[context] modules`: the sentences `reason` returns are written at
  `crates/nvs-db/src/ddl.rs:1004`, which this goal's manifest did not name, so the pack could not
  show what an example would print. The generated goals get their manifest from
  `python tools/dossier.py --emit-goals`, so that is where the pattern belongs.
