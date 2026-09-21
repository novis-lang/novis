# Handoff

## State

Goal `core-csrf-and-3-more`, 9 of 12 items done. `Core\Db::connect` now carries every feature proof
`rule:testing/feature-proofs` names, and `python tools/dossier.py --id 'Core\Db::connect'` prints
`complete.` The three `Core\Db\Column` members are all that is left.

**`[db.notes]` (`nvs.toml:778`) is the recipe the remaining items run on.** It is a fifth SQLite
`:memory:` block, written because `[db.schema]` counts the tables of its own database exactly and
the other three name servers `tests/db/compose.yaml` starts. A program that opens it needs one
`[[app]]` block with `[app.capabilities.db] connect = ["notes"]` and **no `fs` grant at all** — the
`fs.read`/`fs.write` pair the `Core\Db::open` programs carry is `sqlite_settings`' question about a
program-supplied `path`, which `connect` never reaches. `rule:core-classes/db-capabilities` is that
split.

**A `Core\Db\Column` member is reached through `$db->query(…)->columns()`**, and SQLite fills that
description at `crates/nvs-stdlib/src/db/execute.rs:991`, so a `[db.notes]` program reads real
labels with no container. Two answers a proof has to be honest about: `nullable()` is `true` for
every SQLite column, because the driver describes no nullability on a stepped statement
(`execute.rs:1003`), and `type()` is keyed off the column's *declared* type rather than the cell.
`select a, a` describes two columns under one name — the array is never keyed by label.

**Each Column member owes two tests, where `connect` owed one.** `dossier.py --id` reads `tests 0
of 2` for all three: an instance member is credited only by a `covers:` marker, so the Novis half is
a new `.nvst` case carrying one. `tests/conformance/core/db-a-row-is-not-where-a-description-is-read.nvst`
calls all three members and is not that case — it is an `--EXPECTF-ERROR--` case about `Row` having
no `columns`, and it never runs a statement.

`Core\Db::connect` measures 256.2 ns, 6 statements, 0 calls, 3 allocations and 106.4 bytes for one
memoized round, and the declared `calls 0` held. The attack found no bug: twenty thousand names no
block covers, a hundred thousand memoized connects, five hundred connections that do not share, four
names no block can have and ten thousand rows on the connection that stayed open all leave the
runtime standing. One doc fix came out of it — `open_named`'s `# Errors` claimed a `Db\DbError` for
an expired `acquire`, and `pool.rs`'s `wait_for_slot` throws `IOError` and argues why.

## Next group

**Stage: one slice is one feature with all its proofs** — one file set:
`crates/nvs-stdlib/src/db/`, `docs/examples/core/Db-Column/<member>/`,
`tests/hostile/core/Db-Column/<member>/`, `benches/members/core/Db-Column/<member>.nvs`,
`tests/conformance/core/` and `nvs.toml`. `python tools/dossier.py --id '<feature>'` prints the path
of each proof and `--comments <paths>` counts the three bounds before the wrap does. Make every
`crates/` edit of the group **before** the first `--bless` or `--record-perf`: each one buys another
release relink, which the playbook's *Running things* bullet prices.

- [ ] **`Core\Db\Column::name`** — owes about, examples, hostile, perf and two tests.
      `crates/nvs-stdlib/src/db/registry.rs:1545` is the row, and the label it answers is the alias
      the statement wrote. Not tainted, which is that row's own comment: a label comes out of the
      program's own statement, where a value comes from the database.
- [ ] **`Core\Db\Column::type`** — owes the same five. `crates/nvs-stdlib/src/db/registry.rs:1558`,
      answering the `Core\Db\ColumnType` at `crates/nvs-stdlib/src/db/registry.rs:1073`, which is
      the enum that replaces every backend's own type names.
- [ ] **`Core\Db\Column::nullable`** — owes the same five. `crates/nvs-stdlib/src/db/registry.rs:1567`
      is `bool` and never `?bool` on purpose, and `COLUMN_NULLABLE_DOC` is where what the one answer
      means is written down.

## Backlog

- The goal ends with those three items; `docs/agent/loop-goal.md` is the goal.
- `[context] modules` was missing `crates/nvs-stdlib/src/db/column.rs` and `.../db/execute.rs`, which
  every Column item reads. Both copies of the manifest name them now.
- `benches/members/core/Db/connect.nvs` declares `calls 0` and nothing else; its three allocations a
  round were measured rather than predicted, so no count was declared for them.
