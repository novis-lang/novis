# Handoff

## State

Goal `core-db-row`: three of the fourteen members are complete — `bool`, `bytes` and `date` each
carry `about.md`, three examples, an attack, a bench with a recorded figure, a `.nvst` case and a
Rust test. The other eleven owe everything; `python tools/dossier.py --group 'Core\Db\Row'` is the
board. Nothing is blocked.

`rule:types/bytes` sent readers to `Core\Bytes::fromHex()`/`::fromBase64()`, which have never
existed; the fragment now names `Core\Encoding` and the chapter is re-rendered.

## Next group

The next three readers, in the registry's own order. Each is one slice: one feature with all of
`rule:testing/feature-proofs`'s proofs, written the way the three landed ones are — the member's
behaviour is `rule:core-classes/db-column-types`, and the reference card beside the row is what the
description is written from. One file set: `crates/nvs-stdlib/src/db/registry.rs` (the cards),
`crates/nvs-stdlib/src/db/row.rs` (the readers and the `mod tests` their Rust proofs go in),
`nvs.toml` (one `[[app]]` grant per proof program) and the four trees under `core/Db-Row/<member>/`.

- [ ] **`Core\Db\Row::decimal`** — owes about.md, examples, hostile, perf, tests. The claim worth
      pinning is the one `rule:types/decimal` makes: a `DECIMAL` column has no `float` reading and
      the other way round. `crates/nvs-stdlib/src/db/registry.rs:1410`
- [ ] **`Core\Db\Row::instant`** — owes about.md, examples, hostile, perf, tests. Its reader is the
      lookup and a class check, like `date`'s. `crates/nvs-stdlib/src/db/registry.rs:1419`
- [ ] **`Core\Db\Row::time`** — owes about.md, examples, hostile, perf, tests. The third of the time
      readers; `db-row-date-answers-a-calendar-day-and-no-other-time-column.nvst` already asks it
      one of its questions. `crates/nvs-stdlib/src/db/registry.rs:1437`

## Backlog

- `tests/conformance/core/db-a-sqlite-column-reads-back-as-the-type-its-schema-declared.nvst`
  already exercises `string`, `time`, `instant`, `uuid` and `decimal` over a real engine; a
  `covers:` marker there is most of a later slice's Novis-side test — `docs/agent/conventions.md`.
- The eleven members still owed are `decimal`, `float`, `get`, `has`, `instant`, `int`, `string`,
  `time`, `toArray`, `uint` and `uuid` — `python tools/dossier.py --group 'Core\Db\Row'`.
- `[context]` gap: nothing in the pack named the root `nvs.toml`, which every db proof program has
  to be granted in. Add it to `[context] modules` in `docs/agent/loop-goal.toml`.
