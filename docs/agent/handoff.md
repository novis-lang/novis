# Handoff

## State

**Goal `core-db-connection-and-2-more`: three of its eleven members are finished.**
`Core\Db\Connection::close`, `::driver` and `::execute` each carry every feature proof
`rule:testing/feature-proofs` names, and `python tools/dossier.py --verify --group
'Core\Db\Connection'` lists only the other eight. The nine examples and three attacks run green,
and the figures are in `docs/perf/members.ndjson`: `close` 101.7 ns, `driver` 35.5 ns and
`execute` 12.85 µs, the last being one real SQLite insert and 17 allocations.

**Every proof program of this group needs an `[[app]]` block in the root `nvs.toml`** granting
`connect = ["notes"]`, the way `Core\Db::connect`'s and `Core\Db\Column`'s programs already do.
`[db.notes]` is the SQLite `:memory:` block nothing else writes to, so each program creates the
tables it reads.

**Two things the proofs found, both correct behaviour.** A `close` inside a `foreach` over
`stream` makes the walk throw rather than read a connection that went back to the pool
(`crates/nvs-runtime/src/pool.rs:560` only pools what the driver says is clean). `driver()` answers
between two rows while a walk holds the connection, which is what `serverVersion`'s own doc says of
that pair.

## Next group

**Stage: one slice is one feature with all its proofs** — one file set:
`crates/nvs-stdlib/src/db/execute.rs`, `crates/nvs-stdlib/src/db/open.rs`,
`docs/examples/core/Db-Connection/`, `tests/hostile/core/Db-Connection/`,
`benches/members/core/Db-Connection/` and the root `nvs.toml`.

- [ ] **`Core\Db\Connection::query`** — owes about, three examples, an attack, a bench and two
      tests, per `rule:testing/feature-proofs`. `crates/nvs-stdlib/src/db/execute.rs:244` is the
      member and `crates/nvs-stdlib/src/db/registry.rs:258` its row.
- [ ] **`Core\Db\Connection::isOpen`** — same set, and it shares every example receiver with
      `query`. `crates/nvs-stdlib/src/db/open.rs:1272`.
- [ ] **`Core\Db\Connection::executeMany`** — same set.
      `crates/nvs-stdlib/src/db/execute.rs:1375`.

## Backlog

- The five members of this group left after the group above: `queryAs`, `stream`, `streamAs`,
  `transaction`, `serverVersion` — `python tools/dossier.py --verify --group 'Core\Db\Connection'`.
- A `Core\Db\Connection` member that needs a real connection has no Rust-side test of its success
  path; the playbook's `-p nvs-stdlib` bullets own why, and `driver`'s test pins the answer half
  instead — `crates/nvs-stdlib/src/db/open.rs:1468`.
