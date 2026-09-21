# Handoff

## State

Goal `core-csrf-and-3-more`, 8 of 12 items done. `Core\Csrf::issue`, `Core\Csrf::verify`,
`Core\Csv::format`, `Core\Csv::parse`, `Core\Csv::rows`, `Core\Db::open`,
`Core\Db::quoteIdentifier` and now `Core\Db::inList` each carry every feature proof
`rule:testing/feature-proofs` names. `python tools/dossier.py --id 'Core\Db::inList'` prints
`complete.` Four items are left: `Core\Db::connect` and the three `Core\Db\Column` members.

**A `Core\Db::open` proof program stays on the `:memory:` recipe.** It opens
`Core\Db::open({ driver: Core\Db\Driver::Sqlite, path: ':memory:' })`, then runs
`$db->execute(sql, [])`, `$db->executeMany(sql, [[…]])` and
`foreach ($db->query(sql, []) as Core\Db\Row $row)`. Each such program needs one `[[app]]`
block in `nvs.toml` granting `fs.read`, `fs.write` and `db.open`, all three written `true`;
the comment above those blocks is the home of why no list entry can ever match `:memory:`.
A program that only *marks* a list needs no grant at all, which is why
`benches/members/core/Db/inList.nvs` has no block.

**`Core\Db::connect` is a different recipe, and the block it wants does not exist yet.** It
opens a `[db.<name>]` block by name, and of the four in `nvs.toml` only `[db.schema]`
(`nvs.toml:774`) needs no container — the other three are the compose PostgreSQL and SQL
Server. That block's own comment is the argument against sharing it: `examples/schema.nvs`
counts plan steps against every other table in its database. So `connect`'s proofs want a
fifth SQLite `:memory:` block of their own beside it, plus `[app.capabilities.db] connect =
["<name>"]` per program — `nvs.toml:193` is the shape of that grant.

The attack on `inList` found no bug. A hundred thousand markers, twenty thousand refused
empty lists, a list of a hundred thousand values, a thousand statement widths and a marker
nested inside a list all leave the runtime standing; the wide list and the nested marker are
both refused, which is a pass. The bench measures 82.1 ns, 4 statements, 0 calls, 1
allocation and 48 bytes for one marked run of three, and the declared `calls 0` and
`allocations 1` both held. The Rust test asserts what no `.nvst` can reach: an array is a
value with copy-on-write storage, so only Rust can see that the marker carries the caller's
own allocation rather than a copy of it.

`docs/perf/members.ndjson` gained six records for members outside this slice — the playbook
bullet under *Running things* is why.

## Next group

**Stage: one slice is one feature with all its proofs** — one file set:
`crates/nvs-stdlib/src/db/`, `docs/examples/core/Db/<member>/`,
`tests/hostile/core/Db/<member>/`, `benches/members/core/Db/<member>.nvs` and `nvs.toml`.
`python tools/dossier.py --id '<feature>'` prints the path of each proof and `--comments
<paths>` counts the three bounds before the wrap does; a hostile top comment of five lines is
the bound that is easiest to miss.

- [ ] **`Core\Db::connect`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/open.rs:172` The `[db.<name>]` half of § *State* above is the
      whole of what is new here; the rest is the recipe `inList` and `open` already ran on.
      `rule:core-classes/db-capabilities` is what separates its grant from `open`'s.
- [ ] **`Core\Db\Column::name`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1545` Take it with the two below: the three are one
      class and want one set of example programs' worth of setup. Not checked: where a program
      gets a `Core\Db\Column` from, which is the first thing to answer.
- [ ] **`Core\Db\Column::type`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1558`
- [ ] **`Core\Db\Column::nullable`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1567`

## Backlog

- Goal `core-csrf-and-3-more` is done when those four are; nothing else in it is open.
- A `Core\Db::connect` proof that opens a fifth `[db.<name>]` block adds a row to a file every
  program in the repository loads — `nvs.toml`'s own comments are where that choice is argued.
