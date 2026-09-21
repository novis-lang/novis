# Handoff

## State

**Goal `core-csrf-and-3-more` is met.** All four of its stage 2 groups verify green —
`python tools/dossier.py --verify --group` reports `nothing owed` and `0 failed` for
`Core\Csrf`, `Core\Csv`, `Core\Db` and `Core\Db\Column` — and `Core\Db\Column::nullable`,
the last item, now carries every feature proof `rule:testing/feature-proofs` names.
`python tools/verify.py` is 14 of 14 green, `--doc` resolves every link, and both
`python tools/owners.py --closes core-csrf-and-3-more` and `python tools/playbook.py
--closes core-csrf-and-3-more` report that the goal owns nothing.

`::nullable` answers `true` for every column on every driver, and the proofs say so rather
than inventing a `false`: `crates/nvs-stdlib/src/db/execute.rs:1003` and
`crates/nvs-stdlib/src/db/column.rs:101` both write `Value::bool(true)`, because a stepped
statement carries no `not null` flag. It measures 44.9 ns at 0 calls and 0 allocations.

**Two files in the working tree are not this session's** — `crates/nvs-config/src/default.toml`
and `tools/directives.py` carry an uncommitted hand edit teaching the template parser where a
commented key lands. They were left untouched and unstaged; `python tools/verify.py` is green
with them in place.

## Next group

**Stage: one slice is one feature with all its proofs** — the next goal,
`core-db-connection-and-2-more`, installs its own handoff on the switch. Its first three
items share one file set: `crates/nvs-stdlib/src/db/`, `docs/examples/core/Db-Connection/`,
`tests/hostile/core/Db-Connection/` and `benches/members/core/Db-Connection/`.

- [ ] **`Core\Db\Connection::close`** — owes about, three examples, an attack, a bench and two
      tests. `crates/nvs-stdlib/src/db/registry.rs:399` is the row.
- [ ] **`Core\Db\Connection::driver`** — same set, and it shares the receiver, so the second
      slice costs a fraction of the first. `crates/nvs-stdlib/src/db/registry.rs:408` is the row.
- [ ] **`Core\Db\Connection::execute`** — same set.
      `crates/nvs-stdlib/src/db/registry.rs:302` is the row.

A `.nvst` case over a real engine needs no `[[app]]` block: it carries a `--FILE nvs.toml--`
section naming `sqlite` and `:memory:`, and
`tests/conformance/core/db-column-nullable-is-one-answer-for-every-described-column.nvst:25`
is that shape. An example, an attack or a bench does need one `[[app]]` entry in `nvs.toml`
with `[app.capabilities.db] connect = ["notes"]` and no `fs` grant.

## Backlog

- `crates/nvs-db/src/sqlite.rs` is where `SqliteColumn::column_type` maps a declared type name
  onto a case; a `Core\Db\Column` proof reads it and no `[context] modules` pattern names it.
  Add the path if the next goal touches that class again — `docs/agent/loop-goal.toml`.
- The `Core\Db\Row` typed readers are goal `core-db-row`'s; the `as ?string` finding above is
  decided behaviour, not a gap, and needs nothing from them.
