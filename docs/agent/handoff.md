# Handoff

## State

Goal `core-db-row`: seven of the fourteen members are complete. `bool`, `bytes`, `date`, `decimal`,
`time` and `uuid` each carry `about.md`, three examples, an attack, a bench with a recorded figure,
a `.nvst` case and a Rust test; `instant` carries its two tests and its `about.md`, with its three
program proofs skipped for a reason each in `tools/data/dossier-policy.toml`.
`python tools/dossier.py --group 'Core\Db\Row'` is the board.

Seven are left: `string`, `int`, `uint`, `float`, `has`, `get` and `toArray`. The first four are
readers of the shape the seven landed ones have; the last three read the row itself and take a
different set of examples.

`target/release/nvs.exe` was rebuilt this session by `--bless` and then again by `--record-perf`,
because a `crates/` edit landed between them. Make every Rust edit of a group before the first
`--bless`.

## Next group

The next three readers, in the order the registry declares them. Each is one slice: one feature
with all of `rule:testing/feature-proofs`'s proofs, written the way the seven landed ones are — the
member's behaviour is `rule:core-classes/db-column-types`, and the reference card beside the row in
the registry is what `about.md` is written from. One file set:
`crates/nvs-stdlib/src/db/registry.rs` (the cards), `crates/nvs-stdlib/src/db/row.rs` (the readers
and the `mod tests` their Rust proofs go in), `nvs.toml` (one `[[app]]` grant per proof program)
and the four trees under `core/Db-Row/<member>/`.

- [ ] **`Core\Db\Row::string`** — owes about.md, examples, hostile, perf, tests. The claim worth
      pinning is the one the `uuid` case already pins from the other side: a `uuid` column has no
      text reading, so the two disagree in both directions.
      `crates/nvs-stdlib/src/db/row.rs:1031`
- [ ] **`Core\Db\Row::int`** — owes about.md, examples, hostile, perf, tests. Its bound is already
      asserted from Rust by `bigint_unsigned_past_i64_max_reads_uint_and_throws_for_int`, so its own
      Rust proof takes another claim. `crates/nvs-stdlib/src/db/row.rs:1061`
- [ ] **`Core\Db\Row::uint`** — owes about.md, examples, hostile, perf, tests. The same bound from
      the other side: a negative `BIGINT` has no `uint` reading, and `0` is where that stops.
      `crates/nvs-stdlib/src/db/row.rs:1078`

## Backlog

- `Core\Db\Row::float`, `has`, `get` and `toArray` still owe everything — the board above.
- No reader answers a zone-*less* `DATETIME` column: `instant` refuses a `Core\Time\DateTime` by
  class, and the other ten refuse it too, so such a column is reachable only through `get`. Whether
  that is a gap is `rule:core-classes/db-column-types`'s question, not this goal's.
- `docs/examples/core/Db-Row/time/about.md` and `date/about.md` send a reader holding a day-and-time
  column to `Core\Db\Row::instant`, which refuses one unless it carries a zone. Goal
  `plain-comments` sweeps the landed `about.md` files.
