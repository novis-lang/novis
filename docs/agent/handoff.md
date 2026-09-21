# Handoff

## State

Goal `core-db-row`: five of the fourteen members are complete — `bool`, `bytes`, `date`, `decimal`
and `time` each carry `about.md`, three examples, an attack, a bench with a recorded figure, a
`.nvst` case and a Rust test. `python tools/dossier.py --group 'Core\Db\Row'` is the board.

`Core\Db\Row::instant` is the one member of the fourteen whose example, attack and bench cannot be
written the way the others are: SQLite has no zone-carrying column type, so the reader refuses
every column an in-memory block can hold, and every proof program runs on one. Those three are
skip entries in `tools/data/dossier-policy.toml` with a reason each; its two tests are still owed
and are writable, one `.nvst` for the refusals and one Rust test that builds the row itself.

## Next group

`instant`'s two tests first, then the next two readers in the registry's own order. Each is one
slice: one feature with all of `rule:testing/feature-proofs`'s proofs, written
the way the five landed ones are — the member's behaviour is `rule:core-classes/db-column-types`,
and the reference card beside the row is what the description is written from. One file set:
`crates/nvs-stdlib/src/db/registry.rs` (the cards), `crates/nvs-stdlib/src/db/row.rs` (the readers
and the `mod tests` their Rust proofs go in), `nvs.toml` (one `[[app]]` grant per proof program) and
the four trees under `core/Db-Row/<member>/`.

- [ ] **`Core\Db\Row::instant`'s two tests** — the only proofs it owes; its three program proofs
      are skipped with a reason each in `tools/data/dossier-policy.toml`. The Rust one builds the
      row itself with a `Core\Time\Instant` in a column and so reaches the answer, which no program
      on SQLite can. `crates/nvs-stdlib/src/db/row.rs:1151`
- [ ] **`Core\Db\Row::uuid`** — owes about.md, examples, hostile, perf, tests. The claim worth
      pinning is that a `UUID` column and a `TEXT` column holding the same 36 characters are not
      each other: `uuid` refuses the text one and `string` refuses the native one.
      `crates/nvs-stdlib/src/db/registry.rs:1447`
- [ ] **`Core\Db\Row::string`** — owes about.md, examples, hostile, perf, tests. Its Rust proof is
      half-written already: `a_blob_column_and_a_text_column_do_not_read_as_each_other` asserts the
      `string` side of the crossing and carries no `covers:` for it.
      `crates/nvs-stdlib/src/db/row.rs:1717`

## Backlog

- `Core\Db\Row::instant`'s three skip entries come out the day a proof tree can run against a
  driver behind a container — `tools/data/dossier-policy.toml` says so on the block above them.
- On SQLite a `DECIMAL(10,2)` column has NUMERIC affinity, so stored digits pass through a `REAL`
  and a trailing zero does not survive — `crates/nvs-db/src/sqlite.rs:295` argues it and the
  examples are written around it with values that round-trip.
- `Core\Db\Row::has`, `::get` and `::toArray` are the three non-reader members of the fourteen and
  want a different example shape from the readers.
