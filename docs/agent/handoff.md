# Handoff

## State

Goal `core-csrf-and-3-more`, items 1 to 5 of 12 are done: `Core\Csrf::issue`, `Core\Csrf::verify`,
`Core\Csv::format`, `Core\Csv::parse` and now `Core\Csv::rows` each carry every feature proof
`rule:testing/feature-proofs` names, so `Core\Csv` is complete. `python tools/dossier.py --id
'Core\Csv::rows'` prints `complete.`

`Core\Csv::rows`'s five proof programs are the first under `docs/examples/`, `tests/hostile/` and
`benches/members/` that open a file. Each writes its document into `Core\IO::temporaryDir()`'s
answer and reads it back, and each needs its own `[[app]]` block in the repository's `nvs.toml`
granting `fs.read` and `fs.write` — the comment above those five blocks is the home of why the grant
is `true` rather than a path. The two Rust tests the member already had now carry its `covers:`
marker; nothing else about them changed.

The attack found no bug: a quoted field of eight megabytes that never ends, two hundred thousand
records, a record of two hundred thousand fields, a handle closed under the walk and a line break
asked for as the separator all give a record or an error, and the runtime is still standing. The
bench measures 53,493 ns, 6 calls, 24 allocations and 9,919 bytes for one walk of a two-record file,
which is opening, reading and closing it together; the declared `// bench: calls 6` held exactly.

The seven `Core\Db` items are untouched. Nothing is blocked. `crates/nvs-cli/src/service.rs` and two
files under `docs/rules/packaging/` were modified in the working tree by somebody else while this
session ran, and are not staged by it.

## Next group

**Stage: one slice is one feature with all its proofs** — one file set:
`crates/nvs-stdlib/src/db/registry.rs`, `docs/examples/core/Db/<member>/`,
`tests/hostile/core/Db/<member>/`, `benches/members/core/Db/<member>.nvs` and `nvs.toml`.
`python tools/dossier.py --id '<feature>'` prints the path of each proof, `--comments <paths>`
counts the three bounds before the wrap does, and `--bless` and `--record-perf` each rebuild
`target/release/nvs.exe` first, which is three minutes the first time.

- [ ] **`Core\Db::quoteIdentifier`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:192` Take this one first: it checks a name against the
      bare-identifier grammar and answers a string, so no proof of it opens a connection and none of
      its programs needs a block in `nvs.toml`.
- [ ] **`Core\Db::inList`** — owes about, examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:183` What a proof of it costs is not settled: the member
      marks a run of bound values, so an example that shows the expansion may have to run a
      statement. Read `tests/conformance/core/db-in-list-marks-a-run-of-bound-values.nvst` first,
      and if a connection is needed, `[db.<name>]` over SQLite is the cheap one — the playbook's
      `queue_sqlite.rs` bullet is the Rust half of the same choice.

## Backlog

- The five remaining `Core\Db` items of this goal, after the two above — `python tools/dossier.py
  --group` lists them with what each owes.
- `Core\Csv::parse` spends about 11 µs per call before it reads a byte, fixed per call rather than
  per byte — `crates/nvs-stdlib/src/csv.rs`'s first `# Known gaps` item, owner M12.
