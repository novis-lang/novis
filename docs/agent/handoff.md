# Handoff

## State

Goal `core-db-connection-and-2-more` (milestone `dossier`), 13 features. Eight are complete: the six
before this session, plus `Core\Db\Connection::serverVersion` and `Core\Db\Connection::transaction`
in it. Five are left, all of them `Core\Db\Plan` and `Core\Db\Plan\Step`. `python tools/dossier.py
--id '<feature>'` prints what one owes and where each proof goes.

Every proof program of this group opens `[db.notes]`, the SQLite `:memory:` block at the foot of
`nvs.toml` that no other fixture writes to, and each one needs its own `[[app]]` entry granting
`connect = ["notes"]` — the entries sit in one run, ordered by member name. An entry naming a file
that is not on disk is `E0605` and fails *every* program in the tree, so write the file first.

Nothing is blocked. One finding is recorded rather than fixed: `Core\Db\Queryable` is not a type a
program can write, so no function can take a connection and a transaction alike —
`crates/nvs-stdlib/src/db/mod.rs` `# Known gaps` item 2.

## Next group

One slice is one feature with all its feature proofs. These three share a file set:
`crates/nvs-stdlib/src/db/plan.rs`, `crates/nvs-stdlib/src/db/registry.rs`, `nvs.toml`, and the
`core/Db-Plan/` and `core/Db-Plan-Step/` directories in each of the three proof trees.
`rule:testing/feature-proofs` is what they owe. A plan is read from a real statement, so the
programs need a table before they can explain anything.

- [ ] **`Core\Db\Plan::steps`** — owes about, examples, hostile, perf, tests. Take it first: it is
      what produces the `Core\Db\Plan\Step` objects the four members after it read.
      `crates/nvs-stdlib/src/db/plan.rs:81`
- [ ] **`Core\Db\Plan\Step::grade`** — owes about, examples, hostile, perf, tests. The grade is an
      enum case, so an example compares it with `==` rather than printing a name.
      `crates/nvs-stdlib/src/db/plan.rs:90`
- [ ] **`Core\Db\Plan\Step::sql`** — owes about, examples, hostile, perf, tests. An example must not
      print the engine's own plan text, which would pin a SQLite release in a blessed `.out`.
      `crates/nvs-stdlib/src/db/plan.rs:113`

## Backlog

- `Core\Db\Plan\Step::reason` (`crates/nvs-stdlib/src/db/plan.rs:99`) and
  `Core\Db\Plan\Step::isRefused` (`:127`) close the goal after the group above.
- The `Core\Db\Queryable` gap is tagged `— owner: M10` because that is what the sibling item in the
  same `# Known gaps` block uses; no milestone's plan actually covers it, and whether a goal should
  carry it is the user's call — `docs/plan/m10.md`.
- Two landed benches miss the counts they declare — `lang:types/void-never-self-static` declares
  `allocations 2` and does 1.000. Seen in a whole-roster `--record-perf`; `benches/members/lang/types/`.
