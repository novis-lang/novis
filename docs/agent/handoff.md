# Handoff

## State

Goal `core-csrf-and-3-more`, items 1 to 5 and item 9 of 12 are done. `Core\Csrf::issue`,
`Core\Csrf::verify`, `Core\Csv::format`, `Core\Csv::parse`, `Core\Csv::rows` and now
`Core\Db::quoteIdentifier` each carry every feature proof `rule:testing/feature-proofs` names.
`python tools/dossier.py --id 'Core\Db::quoteIdentifier'` prints `complete.`

`Core\Db::quoteIdentifier`'s proofs need no connection and no grant in `nvs.toml`: it checks a name
against the bare-identifier grammar and returns a string. The attack found no bug — sixteen names
carrying SQL, a legal name of eight million letters, the same name with a quote on the end, and four
hundred thousand checks in a row all return a name or throw, and the runtime is still standing. The
bench measures 36.2 ns, 6 statements, 0 calls, 1 allocation and 38.5 bytes per check, and the
declared `calls 0` and `allocations 1` both held. The Rust test asserts what no `.nvst` can reach:
the member's alphabet **is** `nvs_db::is_bare_identifier`, over a sweep of twenty names, so a second
copy of the grammar cannot drift from the schema validator's.

**The six items left all need a live connection, and none of the tree's examples has ever opened
one.** That decision is the next group's first job, not a block: the examples README says an example
needing a database is the wrong example, and for `Core\Db` there is no version that needs neither, so
a SQLite file under `Core\IO::temporaryDir()` is the answer to write down. What is already known is
in the first item below.

`docs/novis.md` and three files under `docs/reference/` were modified in the working tree by somebody
else while this session ran, and are not staged by it.

## Next group

**Stage: one slice is one feature with all its proofs** — one file set:
`crates/nvs-stdlib/src/db/open.rs`, `docs/examples/core/Db/<member>/`,
`tests/hostile/core/Db/<member>/`, `benches/members/core/Db/<member>.nvs` and `nvs.toml`.
`python tools/dossier.py --id '<feature>'` prints the path of each proof, and `--comments <paths>`
counts the three bounds before the wrap does.

- [ ] **`Core\Db::open`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/open.rs:712` Take this one first: its proofs are what settle how
      every later `Core\Db` program gets a connection, and the rest then copy it. Probed and
      compiling already: `Core\Db::open({ driver: Core\Db\Driver::Sqlite, path: $file })` against a
      file under `Core\IO::temporaryDir()`, then `$db->execute(sql, [])` and
      `foreach ($db->query(sql, []) as Core\Db\Row $row)`. It needs three grants in one `[[app]]`
      block per program: `fs` read and write for the temporary directory, `db.open`, and `db.schema`
      for the `CREATE TABLE`. `db.open`'s scope is the *path*
      (`crates/nvs-stdlib/src/db/open.rs:910`), which no checkout can write down for a host
      temporary directory, so the grant is `true` the way the `Core\Csv::rows` blocks in `nvs.toml`
      are, and the comment above them is the home of that reason.
- [ ] **`Core\Db::inList`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/open.rs:1092` The marker expands one `?` into `(?, ?, ?)` at bind
      time, so nothing it does is observable without a statement: its examples are the one above
      plus a list. The attack is the one that matters — an empty list, a list of a hundred thousand
      values, and a nested marker.
- [ ] **`Core\Db::connect`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/open.rs:172` Its grant is a block name rather than a path, the way
      the `examples/db.nvs` block in `nvs.toml` is, so it needs a `[db.<name>]` block of its own
      beside the examples, pointing at a SQLite file.

## Backlog

- `Core\Db\Column::name`, `::nullable` and `::type` (items 10 to 12) owe every proof, and read a
  column off a result set, so they follow the connection pattern the group above settles.
- Two benches declare a count they no longer meet and so record nothing:
  `benches/members/lang/types/void-never-self-static.nvs` declares `allocations 2` and does 1. A
  `--record-perf --force` run names both; `benches/members/README.md` § *What a bench declares* is
  the owner.
- `docs/examples/README.md` § *What an example is* says an example needing a database is the wrong
  example. Once the group above lands, that line owes the `Core\Db` carve-out in one sentence.
