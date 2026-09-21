# Handoff

## State

Goal `core-db-connection-and-2-more` (milestone `dossier`), 13 features. Six are complete:
`Core\Db\Connection::query`, `executeMany` and `isOpen` before this session, and `queryAs`, `stream`
and `streamAs` in it. Seven are left — `serverVersion`, `transaction` and the `Core\Db\Plan`
members. `python tools/dossier.py --id '<feature>'` prints what one owes and where each proof goes.

Every proof program of this group opens `[db.notes]`, the SQLite `:memory:` block at the foot of
`nvs.toml` that no other fixture writes to, and each one needs its own `[[app]]` entry granting
`connect = ["notes"]` — the entries sit in one run, ordered by member name.

Nothing is blocked. One proof found a documented price rather than a bug: a `stream` walk the
program leaves early keeps the connection until the request ends, so `stream`'s second example
bounds its result with `limit` and the playbook carries the trap.

## Next group

One slice is one feature with all its feature proofs. These two share a file set with the three
that landed here: `crates/nvs-stdlib/src/db/registry.rs`, `crates/nvs-stdlib/src/db/open.rs`,
`crates/nvs-stdlib/src/db/transaction.rs`, `nvs.toml`, and the `core/Db-Connection/` directory in
each of the three proof trees. `rule:testing/feature-proofs` is what they owe, and both already have
`.nvst` cases that need only a `covers:` marker.

- [ ] **`Core\Db\Connection::serverVersion`** — owes about, examples, hostile, perf, tests. An
      example must not print the version itself: the string is the engine's own and would pin a
      SQLite release in a blessed `.out`.
      `crates/nvs-stdlib/src/db/registry.rs:421`
- [ ] **`Core\Db\Connection::transaction`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:568`

## Backlog

- The five `Core\Db\Plan` members close this goal after the two above — `python tools/dossier.py
  --group 'Core\Db\Plan'`.
- `[context] modules` did not name `crates/nvs-stdlib/src/db/stream.rs` or `db/row.rs`, where two of
  this session's members live; this session's commits touch both, so the driver's sweep picks them
  up — `docs/agent/loop-goal.toml`.
- If `Core\Db\Stream` ever frees its connection when it is dropped, `stream`'s second example, its
  `about.md` and the playbook bullet all say something that stopped being true —
  `docs/examples/core/Db-Connection/stream/about.md`.
