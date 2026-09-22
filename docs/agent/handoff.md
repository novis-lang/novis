# Handoff

## State

Goal `core-db-transaction-and-1-more`: 7 of the 8 `Core\Db\Transaction` members carry their feature
proofs. `execute`, `executeMany`, `query`, `queryAs`, `stream`, `streamAs` and `rollBack` are
complete, gated and measured; `python tools/dossier.py --gate --group 'Core\Db\Transaction'` names
only `transaction`. The three `Core\Db\Write` members are the rest of the goal.

The streaming pair's deterministic counts say what the hydration costs on top of the walk: 18.00
statements and 87.72 allocations for `stream`, 33.00 and 117.69 for `streamAs`, over the same 17.00
calls and within 600 bytes of each other. `rollBack` is 7.00 statements and 47.06 allocations, and
its `ns/op` is the only figure in the group that is a third of its siblings', because it runs one
statement and gives up rather than reading rows.

`target/release/nvs.exe` is current with the tree, which is what the goal's own dossier check reads
before it judges anything — the acceptance failure after session 0028 was that binary being stale
and nothing else.

What only a `.nvst` can pin here is the connection the walk holds. A proof program cannot get a
second database flow (see the playbook bullet), so an attack abandons at most one walk and that
step is last; a `fromRow` that throws, and the give-up caught inside the work that still does not
commit, are pinned from `tests/conformance/` instead.

## Next group

**Stage: feature proofs for the last `Core\Db\Transaction` member and the `Core\Db\Write` trio** —
one file set: `crates/nvs-stdlib/src/db/registry.rs`, `crates/nvs-stdlib/src/db/bind.rs`, the root
`nvs.toml`, and the proof trees under `core/Db-Transaction/transaction/` and `core/Db-Write/`. What
`transaction` owns that no member above it does is the nesting rule: a transaction opened inside a
transaction is one unit of work and not two, which is what its `.nvst` and its attack are written
around. `Core\Db\Write`'s three readers share one receiver and one statement, so their examples and
their bench are cheapest written together after it.

- [ ] **`Core\Db\Transaction::transaction`** — owes examples, hostile, perf, tests. The registry row
      is `TRANSACTION_ROW`, shared with `Core\Db\Connection`, at
      `crates/nvs-stdlib/src/db/registry.rs:722`; the body it names is
      `crates/nvs-stdlib/src/db/transaction.rs:505`. `rule:core-classes/db-transactions` and
      `rule:testing/feature-proofs`.
- [ ] **`Core\Db\Write::affected`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1486`. `rule:testing/feature-proofs`.
- [ ] **`Core\Db\Write::changed`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1495`. `rule:testing/feature-proofs`.
- [ ] **`Core\Db\Write::lastId`** — owes examples, hostile, perf, tests.
      `crates/nvs-stdlib/src/db/registry.rs:1504`. `rule:testing/feature-proofs`.

## Backlog

- A reason of a hundred million characters reaches the memory ceiling, and the `FATAL` reports the
  bytes *held* rather than the allocation that was refused — `crates/nvs-stdlib/src/db/mod.rs`'s
  `# Known gaps` is where that belongs if it is worth a line.
- `Core\Db\Stream` registers no members, so an abandoned walk cannot be released before the request
  ends; the three `about.md` files under `core/Db-Transaction/` say so, and nothing else does.
- `benches/members/core/Db-Transaction/queryAs.nvs`'s top comment is a 28-word sentence; goal
  `plain-comments` owns the landed files.
