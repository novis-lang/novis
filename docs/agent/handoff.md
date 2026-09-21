# Handoff

## State

Goal `core-db-row`: ten of the fourteen members are complete. `bool`, `bytes`, `date`, `decimal`,
`int`, `string`, `time`, `uint` and `uuid` each carry `about.md`, three examples, an attack, a
bench with a recorded figure, a `.nvst` case and a Rust test; `instant` carries its two tests and
its `about.md`, with its three program proofs skipped for a reason each in
`tools/data/dossier-policy.toml`. `python tools/dossier.py --group 'Core\Db\Row'` is the board.

Four are left: `float`, `has`, `get` and `toArray`. `float` is the last reader of the shape the
nine landed ones have; the other three read the row itself and take a different set of examples.

`target/release/nvs.exe` is rebuilt by `--bless` and again by `--record-perf` whenever a `crates/`
edit landed between them, so a group of slices pays that twice per slice unless every Rust edit is
made before the first `--bless`.

## Next group

The last reader and then the three members that read the row itself. Each is one slice: one
feature with all of `rule:testing/feature-proofs`'s proofs, written the way the ten landed ones
are — the member's behaviour is `rule:core-classes/db-column-types`, and the reference card beside
the row in the registry is what `about.md` is written from. One file set:
`crates/nvs-stdlib/src/db/registry.rs` (the cards), `crates/nvs-stdlib/src/db/row.rs` (the readers
and the `mod tests` their Rust proofs go in), `nvs.toml` (one `[[app]]` grant per proof program)
and the four trees under `core/Db-Row/<member>/`.

- [ ] **`Core\Db\Row::float`** — owes about.md, examples, hostile, perf, tests. Its card is
      `FLOAT`, `REAL` and `DOUBLE` and nothing else, so the claim is the one `int`'s case pins from
      the other side: a `decimal` column holds an exact number and has no `float` reading.
      `crates/nvs-stdlib/src/db/row.rs:1096`
- [ ] **`Core\Db\Row::has`** — owes about.md, examples, hostile, perf, tests. It answers a `bool`
      for a name rather than throwing, so its claim is the pair: `has` is `false` exactly where the
      typed readers throw, and `true` for a column that is NULL in this row.
      `crates/nvs-stdlib/src/db/row.rs:994`
- [ ] **`Core\Db\Row::get`** — owes about.md, examples, hostile, perf, tests. It is the universal
      path the typed readers' refusals name, so its examples are the ones that reach a column no
      typed reader answers. `crates/nvs-stdlib/src/db/row.rs:1004`

## Backlog

- `Core\Db\Row::toArray` still owes everything, and is the fourth of the group above —
  `crates/nvs-stdlib/src/db/row.rs:1022`.
- The pack's `[context] modules` names only `db/registry.rs` and `db/row.rs`, so a Rust proof
  building a `Value` of a given tag pays one `peek.py` for the constructor's spelling in
  `crates/nvs-runtime/src/value.rs`. Adding that path closes it.
- No reader answers a zone-*less* `DATETIME` column: `instant` refuses a `Core\Time\DateTime` by
  class, and the other ten refuse it too, so such a column is reachable only through `get`. Whether
  that is a gap is `rule:core-classes/db-column-types`'s question, not this goal's.
- `docs/examples/core/Db-Row/time/about.md` and `date/about.md` send a reader holding a day-and-time
  column to `Core\Db\Row::instant`, which refuses one unless it carries a zone. Goal
  `plain-comments` sweeps the landed prose and is where that is fixed.
