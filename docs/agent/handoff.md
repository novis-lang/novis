# Handoff

## State

Goal `core-db-rows-and-1-more` — the generated dossier goal over `Core\Db\Rows` and `Core\Db\Schema`
— has 5 of its 11 items landed: `Core\Db\Rows::all`, `::column`, `::columns`, `::count` and `::first`
each carry their feature proofs, and `python tools/dossier.py --id '<feature>'` prints `complete.` for
every one of them. Nothing is blocked. One item of the `Rows` half is left, `::value`; the other five
are all of `Core\Db\Schema`.

Three things the next session should not re-derive. `result_over` at the tail of
`crates/nvs-stdlib/src/db/row.rs`'s `mod tests` builds a `Core\Db\Rows` over exactly two rows with no
class and no described columns; a result with a described column set, or with no rows at all, is
`crate::instance::build(&ROWS, [rows, Value::null(), columns])` written out, which is what the three
Rust cases landing here needed. A conformance case that reaches a real database carries its own
`--FILE nvs.toml--` holding `[capabilities.db] connect = ["main"]` and a `[db.main]` sqlite `:memory:`
block, or it refuses at `Core\Db::connect`. And a perf record is keyed on the implementing file's text
with its trailing `mod tests` cut off (`benches/members/README.md` § *When a figure is re-measured*),
so appending a Rust case to `row.rs` does not stale a figure already taken.

## Next group

**One file set for all three:** `crates/nvs-stdlib/src/db/registry.rs` (the reference card),
`nvs.toml` (one `[[app]]` per runnable proof) and each feature's own proof trees. The member itself
moves out of `row.rs` and into `schema.rs` after the first item, which is where the `Rows` half of the
goal ends. One slice is one feature with all of `rule:testing/feature-proofs`.

- [ ] **`Core\Db\Rows::value`** — owes examples, hostile, perf, tests. The card is
      `crates/nvs-stdlib/src/db/registry.rs:1236` and the member `crates/nvs-stdlib/src/db/row.rs:867`;
      its doc comment is the claim worth pinning, that an empty result and a NULL column are both
      `null` and are deliberately not told apart, so `count()` is what a caller asks instead.
- [ ] **`Core\Db\Schema::fromArray`** — owes examples, hostile, perf, tests. The card is
      `crates/nvs-stdlib/src/db/registry.rs:1608` and the member
      `crates/nvs-stdlib/src/db/schema.rs:185`; it reads the canonical array form, so the attack is a
      malformed one.
- [ ] **`Core\Db\Schema::toArray`** — owes examples, hostile, perf, tests. The card is
      `crates/nvs-stdlib/src/db/registry.rs:1618` and the member
      `crates/nvs-stdlib/src/db/schema.rs:200`; written with `fromArray` because the round trip is one
      claim and both halves share the fixture.

## Backlog

- `Core\Db\Schema::planAgainst`, `::applySafe` and `::applyIncludingRisky` close this goal —
  `docs/agent/loop-goal.md`.
- `benches/members/core/Db-Rows/columns.nvs` and `first.nvs` declare only `// bench: calls 0`, as
  their landed siblings do; the measured 0 allocations for `columns` and 1 for `first` are in
  `docs/perf/members.ndjson` and could be declared when either is next re-measured.
