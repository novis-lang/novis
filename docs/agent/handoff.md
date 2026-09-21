# Handoff

## State

Goal `core-db-transaction-and-1-more`: 4 of the 8 `Core\Db\Transaction` members carry their feature
proofs. `execute`, `executeMany`, `query` and `queryAs` are complete, gated and measured, and each
one's deterministic counts land on `Core\Db\Connection`'s own to the third decimal — 92.045
allocations for both `query`s, 132.067 and 34.004 statements for both `queryAs`s — which is what one
shared body predicts (`rule:classes/no-traits`). Only the advisory clock differs.

What is provable from Rust is still the scope guard alone: no `-p nvs-stdlib` test can build an
`nvs_db::Connection`. `query`'s case drives `crate::db::execute::queried_rows` and pins that the
guard answers *before* the statement's own `timeout` is read, with the option refusal asserted
separately so the order is the claim. `queryAs`'s is agreement over the registry:
`crate::registry::WRITTEN_CLASS_MEMBERS` carries every written member under both receivers, which is
what puts three constants ahead of the receiver the body reads at `args[3]`.

Every proof program needs an `[[app]]` block in the root `nvs.toml` granting
`db.connect = ["notes"]`, and each block's `entry` must exist on disk: a block naming a file that is
not there is `E0605` on *every* program run, not just that one.

A buffered read is not held to the request's memory ceiling — the `Db-Connection/query` attack marks
that gap on `crates/nvs-stdlib/src/db/mod.rs`, and half a million hydrated objects measured 318 MB
peak under this tree's 256M limit, so the `queryAs` attack's flood step asserts survival only.
Nothing is blocked.

## Next group

**Stage: feature proofs for `Core\Db\Transaction`'s streaming members** — one file set:
`crates/nvs-stdlib/src/db/registry.rs`, `crates/nvs-stdlib/src/db/bind.rs`, the root `nvs.toml`, and
the four proof trees under `core/Db-Transaction/`. Each slice is one member with all of
`rule:testing/feature-proofs`' artefacts. What these two own that the buffered pair did not is § 4's
connection-busy rule: a stream holds the very connection the `COMMIT` has to go out on, so a second
statement through the same transaction is a `LogicError` rather than a queued statement — that is
the claim their `.nvst` cases and attacks are written around.

- [ ] **`Core\Db\Transaction::stream`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:686`
- [ ] **`Core\Db\Transaction::streamAs`** — owes examples, hostile, perf, tests; the written-class
      half, so its Rust case has the `args[3..]` slice the roster case already pins.
      `crates/nvs-stdlib/src/db/registry.rs:705`
- [ ] **`Core\Db\Transaction::rollBack`** — owes examples, hostile, perf, tests; this is the member
      the driver's acceptance check names. `crates/nvs-stdlib/src/db/registry.rs:724`

## Backlog

- `Core\Db\Transaction::transaction` is the group's eighth member and owes everything — a nested
  transaction is a savepoint on SQLite (`tests/conformance/core/db-a-sqlite-transaction-nests-as-a-savepoint.nvst`).
- A buffered read outruns the request's memory ceiling: `crates/nvs-stdlib/src/db/mod.rs` § *Known gaps*.
