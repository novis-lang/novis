# Handoff

## State

Goal `core-db-connection-and-2-more` (milestone `dossier`), 13 features. Eleven are complete: the
eight before this session, plus `Core\Db\Plan::steps`, `Core\Db\Plan\Step::grade` and
`Core\Db\Plan\Step::sql` in it. Two are left, both on `Core\Db\Plan\Step`. `python tools/dossier.py
--id '<feature>'` prints what one owes and where each proof goes.

Every proof program of this group opens `[db.notes]`, the SQLite `:memory:` block at the foot of
`nvs.toml` that no other fixture writes to, and each one needs its own `[[app]]` entry granting
`connect = ["notes"]` — the entries sit in one run, ordered by member name. An entry naming a file
that is not on disk is `E0605` and fails *every* program in the tree, so write the file first. A
program that applies a plan also holds `schema = ["notes"]`; planning itself is an ordinary read.

Nothing is blocked. One finding is recorded rather than fixed: `Core\Db\Queryable` is not a type a
program can write, so no function can take a connection and a transaction alike —
`crates/nvs-stdlib/src/db/mod.rs` `# Known gaps` item 2.

## Next group

One slice is one feature with all its feature proofs. These two share a file set:
`crates/nvs-stdlib/src/db/plan.rs`, `nvs.toml`, the `core/Db-Plan-Step/` directories in the three
proof trees, and `tests/conformance/core/`. `rule:testing/feature-proofs` is what they owe. The two
members read a slot each, as the three landed this session do, so the fixtures are the same: a
schema value planned against an empty `[db.notes]`, with one table created by hand when the plan
needs a report in it.

- [ ] **`Core\Db\Plan\Step::reason`** — owes about, examples, hostile, perf, tests. The reason is
      the emitter's own sentence, so an example prints it and never matches on its words; a blessed
      `.out` holding one is what pins it. `crates/nvs-stdlib/src/db/plan.rs:99`
- [ ] **`Core\Db\Plan\Step::isRefused`** — owes about, examples, hostile, perf, tests. A refused
      step is the report a plan carries and never runs, so the example that does want the drop runs
      the step's own `sql()` through `Core\Db\Connection::execute`.
      `crates/nvs-stdlib/src/db/plan.rs:127`

## Backlog

- The two slices above are the goal's last features; `docs/agent/loop-goal.md` names the three
  gates a `DONE` claim owes after them.
- `Core\Db\Queryable` has no writable spelling — `crates/nvs-stdlib/src/db/mod.rs` `# Known gaps`.
- No `Locking` step is reachable on SQLite, so the grade's middle case is pinned by the registry's
  roster alone — `crates/nvs-stdlib/src/db/registry.rs:1912`.
