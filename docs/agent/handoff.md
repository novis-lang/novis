# Handoff

## State

Goal `core-csrf-and-3-more`, 7 of 12 items done. `Core\Csrf::issue`, `Core\Csrf::verify`,
`Core\Csv::format`, `Core\Csv::parse`, `Core\Csv::rows`, `Core\Db::quoteIdentifier` and now
`Core\Db::open` each carry every feature proof `rule:testing/feature-proofs` names. `python
tools/dossier.py --id 'Core\Db::open'` prints `complete.`

**The connection question the last handoff left open is settled, and the answer is `:memory:`.**
A `Core\Db` proof program opens `Core\Db::open({ driver: Core\Db\Driver::Sqlite, path: ':memory:' })`
and then runs `$db->execute(sql, [])`, `$db->executeMany(sql, [[…]])` and
`foreach ($db->query(sql, []) as Core\Db\Row $row)`. Each program needs one `[[app]]` block in
`nvs.toml` granting `fs.read`, `fs.write` and `db.open`, all three written `true`; the comment above
those blocks is the home of why no list entry can ever match `:memory:`. The trailing options bag is
positional — `Core\Db::open({…}, { shared: false })`, never `shared: false`.

A database file under `Core\IO::temporaryDir()` was tried first and is worse: a pooled connection is
returned to its pool rather than closed, so the file is still open when the process sweeps its owned
root, and every run leaves a directory behind and a warning naming it.

The attack found no bug — twenty thousand refused opens, a hundred thousand memoized ones, a path of
a hundred thousand letters and ten thousand statements afterwards all leave the runtime standing.
The bench measures 304.8 ns, 6 statements, 0 calls, 4 allocations and 140.4 bytes for one memoized
open, and the declared `calls 0` and `allocations 4` both held. The Rust test asserts what no `.nvst`
can reach: a program sees the first refusal and nothing after it, so only Rust can say that
`db.open` is asked before `fs.read` and that both are asked before the path is opened.

`crates/nvs-runtime/src/routes.rs`, `crates/nvs-stdlib/src/router.rs` and three `.nvst` files under
`tests/conformance/core/` were modified in the working tree by somebody else while this session ran,
and are not staged by it.

## Next group

**Stage: one slice is one feature with all its proofs** — one file set:
`crates/nvs-stdlib/src/db/open.rs`, `docs/examples/core/Db/<member>/`,
`tests/hostile/core/Db/<member>/`, `benches/members/core/Db/<member>.nvs` and `nvs.toml`. The
`:memory:` recipe above is the whole of what these need; `python tools/dossier.py --id '<feature>'`
prints the path of each proof and `--comments <paths>` counts the three bounds before the wrap does.

- [ ] **`Core\Db::inList`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/open.rs:1092` It needs no connection of its own — it builds the
      carrier a statement expands — but its examples want one to be worth reading, so take it next
      while the recipe above is cheap. The driver's acceptance check is failing on this member.
- [ ] **`Core\Db::connect`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/open.rs:172` Its proofs cannot use `:memory:` through a settings
      literal: `connect` takes a `[db.<name>]` block, so they run on `[db.schema]` at the foot of
      `nvs.toml`, which is already that database, and their grant is `connect = ["schema"]` the way
      the `Db-DbError` example blocks above it are.

## Backlog

- Three more `Core\Db` members still owe every proof — `python tools/dossier.py --owed --group 'Core\Db'`.
- A pooled SQLite connection is returned rather than closed, so a database opened under
  `Core\IO::temporaryDir()` keeps the process from sweeping its own owned root. No module doc records
  it: no milestone in `docs/implementation-plan.md`'s table fits it as an owner, and `owners.py`
  refuses a goal slug.
- `Core\Db::open` asks `fs.read` and `fs.write` about a `path` of `:memory:`, so a program that opens
  no file at all must be granted the whole filesystem. `nvs.toml`'s comment above the `Core\Db::open`
  blocks says why the alternative — a check that knows which paths SQLite reads as memory — is worse.
- `docs/perf/members.ndjson` carries two rows for `Core\Db::open`: the first measured before the bench
  declared its counts, the second after.
