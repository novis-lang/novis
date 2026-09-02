# Handoff

## State

**ADR 0135 § 2's arm selection is the checker's rule.** `Ty::CoreShape` carries a
`CoreShape { fields, arms }` — § 3's merged list, which is the ABI, beside § 2's arms, which are
what a written literal is held to. `nvs_types::expr::args` checks in two passes: every value
against its merged slot (the union of the arms' declarations, so a value some arm accepts is not
refused before its arm is known), then `select_arm` picks the arm that accepts on keys *and*
values, then `report_against_arm` reports against that one. Where no arm accepts, the arm with the
fewest mistakes is the one named, which is what makes `{driver: Sqlite, path, host}` read as
"`host` is not a key of this form" rather than as the server arm's four missing keys. No new
diagnostic code: E0454 for a key outside the selected arm, E0402 for one it requires, E0401 for a
value it refuses — and the E04xx/E07xx bands are both full, so reuse was forced as well as right.

**A bag is the one-arm case and takes the same path**, its single arm being the merged list
itself, so nothing about ADR 0063 R2 changed. `nvs-ir` still flattens the merged list alone: which
arm was selected is a checking question, and § 3's ABI is one argument per merged slot either way.

**`nvs_stdlib::db`'s known gap 1 is now one half, not two**: `open` still files its connection
with no lease, because § 13 keys an `open` pool on a hash of every settings field and
`nvs_runtime::pool::Ticket::for_block` takes a block *name*. Within a request § 2's memo holds.

## Next group

**§ 13's pool for `open`, then the two arms' own conformance depth. File set:
`crates/nvs-runtime/src/pool.rs` with `crates/nvs-stdlib/src/db.rs`.** The first item is the
group's weight; the second is over landed work and shares the second file.

- [ ] **A ticket keyed on the settings hash, so `open` pools** (0067 § 13).
      `crates/nvs-runtime/src/pool.rs:1` is the pool and `Ticket::for_block`'s block-name key;
      `crates/nvs-stdlib/src/db.rs:2995` is the merged-slot list `settings_key` hashes, and
      `crates/nvs-stdlib/src/db.rs:64` is known gap 1, which this closes. The key is § 2's — the
      hash, scoped to the configuration generation it was read from, exactly as a named block's is.
- [ ] **`{shared: false}` still draws from and returns to that pool** (0067 § 13).
      `crates/nvs-stdlib/src/db.rs:64` — § 13 says the option bypasses memoization within the
      request, never pooling across requests, and nothing asserts the second half.
- [ ] **`open`'s reset is the one a failed reset destroys** (0067 § 13).
      `crates/nvs-stdlib/src/db.rs:64` — a `.nvst` case over the connection an `open` returned,
      beside the `connect` cases that already hold it.

## Backlog

- `Core\Db::open`'s SQLite arm opens no file — `nvs_stdlib::db` known gap 2.
- `queryAs`'s body over an `open`ed connection — `nvs_stdlib::db` known gap 8.
- `stream`/`streamAs`, `close` and § 18's three readonly properties — `nvs_stdlib::db`'s `CONNECTION`.
- Spec § 18's `open` row and its Q column against the landed arms — docs/spec/01-core-library.md.
