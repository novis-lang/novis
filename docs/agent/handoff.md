# Handoff

## State

**ADR 0084 § 1's roster is complete** — `Core\Queue::push`, `::status`, `::cancel` and `::stats` are
live in `crates/nvs-stdlib/src/queue.rs`, with `Core\Queue\Id`, `Core\Queue\Stats` and
`Core\Queue\State` in the registry. That module's known gaps say what the surface still owes.

**An ADR 0036 shape now encodes as a JSON object**, keyed by its field names in the sorted order its
synthesized class carries. `nvs_runtime::ClassDesc::is_shape` is the test — the `$shape{…}` label is
the only mark a shape class has, and every structural test would also admit a declared class that
never opted in. ADR 0071 § 7 owns the decision and why it is not a hole in § 1's written opt-in; the
encoding is structural rather than derived because a shape class is keyed on field *names* alone and
so carries no per-field wire type. A `secret` inside a shape is refused at the call site by ADR 0033
§ 4's sink (E0791) and cannot be refused at run time — `record_shape_class` marks no slot `secret` —
so `tests/conformance/reject/json-encode-refuses-a-secret.nvst` now pins that position too.

**Stage 8's acceptance line moved again.** `examples/queue.nvs` gets past line 54 and now fails on
configuration: `nvs.toml` writes no `[queue]` block, so `push` has nothing naming the database. That
is the first move of the worker slice, not a separate one — `payload_of` runs before the block is
resolved, which is why it was hidden. Past that the fixture still needs a worker for `claimed 1`,
`ran` and `retried`, and § 4's claim statement is unwritten.

**`nvs queue migrate` is bigger than it looks, and its check's `want` disagrees with the tree.**
`crates/nvs-cli` depends on `nvs-config` and `nvs-stdlib` and **not** on `nvs-db`, and no CLI path
opens a connection at all — a migrate that really runs needs a connection outside a request context,
which is a design call the slice has to make and record. The check wants `dead_letter` in the output
while `DEAD_TABLE` is `nvs_dead_jobs`, so either the printed line labels the table or the constant
changes; decide it with ADR 0084 § 2 open.

**`orient.py`'s pack was short in the same places again**: `[context] modules` names no
`nvs-config/src/*` and owes `nvs-stdlib/src/queue.rs`; `[context] adrs` printed ADR 0067 §§ 1, 9 and
13 but this item lived in ADR 0071 § 7 and ADR 0036 § 2, both sliced by hand.

## Next group

**The queue's two remaining ends, over `crates/nvs-cli/src/main.rs`, `crates/nvs-stdlib/src/queue.rs`
and `nvs.toml`.**

- [ ] **`nvs queue migrate` creates both of § 2's tables** — ADR 0084 § 2. The subcommand enum is
      `crates/nvs-cli/src/main.rs:139` and `ConfigCommand` at `crates/nvs-cli/src/main.rs:331` is the
      sibling to copy; the table names are `crates/nvs-stdlib/src/queue.rs:102` and
      `crates/nvs-stdlib/src/queue.rs:111`, and `INSERT` at `crates/nvs-stdlib/src/queue.rs:133` is
      the column list the jobs DDL owes. Decide where the DDL lives and whether `--dry-run` is the
      only offline half; the `want` mismatch above is part of this item.
- [ ] **`nvs.toml` gets § 2's `[queue]` block, so the fixture reaches a worker** — ADR 0084 § 2.
      `examples/queue.nvs:22` says what it needs (`workers`, `connection` naming `[db.main]`), and
      `crates/nvs-stdlib/src/queue.rs:847` is the one reading of the block. Check the other fixtures
      still run: every `nvs run` in the sweep reads this file.
- [ ] **§ 4's claim statement, so a worker can take a job** — ADR 0084 § 4. It sits beside
      `crates/nvs-stdlib/src/queue.rs:157`'s `STATUS` and `crates/nvs-stdlib/src/queue.rs:176`'s
      `CANCEL`, and `queue_statements_agree_with_the_state_enum` holds its ordinals.

## Backlog

- `nvs-db`'s four other drivers: PostgreSQL is the only wire I/O on disk — `docs/plan/m8.md`.
- Stage 5's remaining three checks — `docs/agent/loop-goal.toml`, stage 5.
- `Core\Db::open` still waits on a shape-parameter type — ADR 0067 § 2.
- `decodeAs<T>` into an inline shape, the other half of what landed here — spec § 6.
