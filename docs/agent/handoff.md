# Handoff

## State

Goal `core-csrf-and-3-more`, items 1 to 4 of 12 are done. `Core\Csrf::issue`, `Core\Csrf::verify`,
`Core\Csv::format` and `Core\Csv::parse` each carry every feature proof
`rule:testing/feature-proofs` names: `about.md`, three examples with blessed `.out` files, one
attack, one bench, and a Rust test carrying the member's own `covers:` marker. Both `Core\Csv`
figures are in `docs/perf/members.ndjson`, and the `calls 0` each bench declares held.

Neither attack found a bug: every field built out of the dialect's own bytes survives `format` then
`parse` unchanged, every disallowed dialect is refused, and a document an upload form could send —
an unclosed quoted field of eight megabytes, a record of two hundred thousand fields — is read with
the runtime still standing. **The bench did find something**, and it is written up as
`crates/nvs-stdlib/src/csv.rs`'s first `# Known gaps` item, owner M12: `parse` spends about 11 µs
per call before it reads a byte, against `format`'s 120 ns for the same table, and the cost is fixed
per call rather than per byte — an empty document costs the same.

`Core\Csv::rows` (item 5) and the seven `Core\Db` items are untouched. Nothing is blocked.

## Next group

**Stage: one slice is one feature with all its proofs** — one file set:
`crates/nvs-stdlib/src/csv.rs`, `docs/examples/core/Csv/rows/`, `tests/hostile/core/Csv/rows/`,
`benches/members/core/Csv/rows.nvs` and `nvs.toml`. `python tools/dossier.py --id '<feature>'`
prints the path of each proof, and `--comments <paths>` counts the three bounds before the wrap
does.

- [ ] **`Core\Csv::rows`** — owes examples, hostile, perf, tests. `crates/nvs-stdlib/src/csv.rs:223`
      Every proof program has to open a file, which no program under `docs/examples/`,
      `tests/hostile/` or `benches/members/` does yet; the capability block each one needs is the
      playbook bullet above, and the member is reached as `Core\IO::open($path,
      Core\IO\FileMode::Read)` and then `foreach (Core\Csv::rows($file) as array<string> $record)`.
- [ ] **`Core\Db::inList`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:183`
- [ ] **`Core\Db::quoteIdentifier`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:192`

The two `Core\Db` members above are the pair that needs no connection, so they share
`crates/nvs-stdlib/src/db/registry.rs` and nothing else; take them together, and leave
`Core\Db::connect` and `Core\Db::open` to the session that works out how a proof program reaches a
database.

## Backlog

- `Core\Db::connect`, `Core\Db::open`, `Core\Db\Column::name`, `::nullable`, `::type` — the rest of
  this goal's twelve, `docs/agent/loop-goal.md`.
- A `Core\Db` proof needs a reachable database; `crates/nvs-stdlib/tests/queue_sqlite.rs` is the one
  driver that needs no container, and `nvs_db::sqlite::open` is how it gets one.
