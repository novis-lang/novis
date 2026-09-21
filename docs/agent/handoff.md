# Handoff

## State

Goal `core-db-transaction-and-1-more`: 2 of the 8 `Core\Db\Transaction` members carry their feature
proofs. `execute` and `executeMany` are complete, gated and measured, and both land on
`Core\Db\Connection`'s own per-op counts — which is what one shared body predicts
(`rule:classes/no-traits`).

The Rust half of a `Core\Db\Transaction` member lives in `crates/nvs-stdlib/src/db/bind.rs`'s
`mod tests`, and the scope guard is what is provable there: no `-p nvs-stdlib` test can build an
`nvs_db::Connection`, so `transaction_of`'s refusal, reached through `statement_of` and `batch_of`,
is the one thing that tells the two receivers apart. Everything needing a live statement is the
conformance case's, on SQLite `:memory:` through a `--FILE nvs.toml--` section.

Every proof program also needs an `[[app]]` block in the root `nvs.toml` granting
`db.connect = ["notes"]`. Nothing is blocked.

## Next group

**Stage: feature proofs for `Core\Db\Transaction`'s read members** — one file set:
`crates/nvs-stdlib/src/db/registry.rs`, `crates/nvs-stdlib/src/db/bind.rs`, the root `nvs.toml`, and
the four proof trees under `core/Db-Transaction/`. Each slice is one member with all of
`rule:testing/feature-proofs`' artefacts, and `Core\Db\Connection`'s landed twin is the shape to
depart from rather than to copy.

- [ ] **`Core\Db\Transaction::query`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:617`
- [ ] **`Core\Db\Transaction::queryAs`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:641`
- [ ] **`Core\Db\Transaction::stream`** — owes examples, hostile, perf, tests. § 4's connection-busy
      rule is this one's own subject: a stream inside a transaction holds the connection the
      `COMMIT` goes out on. `crates/nvs-stdlib/src/db/registry.rs:685`

## Backlog

- `Core\Db\Transaction::streamAs`, `::transaction` and `::rollBack` still owe every proof —
  `python tools/dossier.py --gate --group 'Core\Db\Transaction'` is the list.
- A perf figure for this class is re-measured whenever `crates/nvs-stdlib/src/db/registry.rs` moves,
  so the group's last session runs `--record-perf --group 'Core\Db\Transaction'` once.
