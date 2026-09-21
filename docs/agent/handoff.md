# Handoff

## State

Goal `core-db-connection-and-2-more` (milestone `dossier`), 13 features. Three are complete:
`Core\Db\Connection::query` landed before this session, and `executeMany` and `isOpen` landed in it.
Ten are left, all of them `Core\Db\Connection` or `Core\Db\Plan` members — `python tools/dossier.py
--id '<feature>'` prints what one owes and where each proof goes.

Every proof program of this group opens `[db.notes]`, the SQLite `:memory:` block at the foot of
`nvs.toml` that no other fixture writes to, and each one needs its own `[[app]]` entry granting
`connect = ["notes"]` — the entries sit in one run, ordered by member name.

Nothing is blocked. No proof written this session found a bug, so this goal has recorded no new
known gap.

## Next group

One slice is one feature with all its feature proofs. These three share one file set:
`crates/nvs-stdlib/src/db/registry.rs`, `crates/nvs-stdlib/src/db/execute.rs`,
`crates/nvs-stdlib/src/db/stream.rs`, `nvs.toml`, and the `core/Db-Connection/` directory in each of
the three proof trees. They are the members that answer a walk or a written type, so the second and
third cost a fraction of the first — `rule:testing/feature-proofs` is what they owe.

- [ ] **`Core\Db\Connection::queryAs`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:281`
- [ ] **`Core\Db\Connection::stream`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:346`
- [ ] **`Core\Db\Connection::streamAs`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:376`

What this session did, as the shape to copy: the examples and the attack are run with
`target/debug/nvs.exe run <file>` and blessed with `python tools/dossier.py --bless <files>`; a
`.nvst`'s frozen `--EXPECTF-ERROR--` line and column come from `python tools/try.py <the .nvst>`,
which numbers the `--FILE--` block exactly as the runner does; the bench figure needs `cargo build
--release -p nvs-cli` before `python tools/dossier.py --record-perf --only '<feature>'`.

## Backlog

- `Core\Db\Connection::serverVersion` and `::transaction`, then the five `Core\Db\Plan` and
  `Core\Db\Plan\Step` members — the rest of this goal, `docs/agent/loop-goal.md` § *The item list*.
- `[context] adrs` names no section of ADR 0067, and § 4 (the batch) and § 18 (the member table) are
  what these members are written against; the registry's own reference cards carried enough this
  time.
- A `-p nvs-stdlib` test still cannot build an `nvs_db::Connection`, so every `Core\Db\Connection`
  member's Rust proof has to be asserted one seam below the member —
  `crates/nvs-stdlib/src/db/open.rs`'s and `execute.rs`'s new cases are the two shapes that work.
