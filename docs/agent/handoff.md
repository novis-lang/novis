# Handoff

## State

**Goal 21, item 8 is closed.** `examples/stream.nvs` is the program its own comment describes and
the frozen `exact` check at `docs/agent/loop-goal.toml:4130` gets its three lines at exit 0; the
root `nvs.toml` grants it `db.connect = ["main"]` beside the `db.nvs` and `transaction.nvs` blocks.
Stage 5's `cargo-named` check ahead of it was already green. Conformance is 1553, unchanged.

**Line 2 of that check is the *program's* count, and both the fixture and the check's comment now
say so.** No `Core` member answers "how many rows is the connection holding", so `rows held at once`
counts what the file itself has in hand — a `array<Row> $held` declared inside the loop body, which
a fixture that accumulated the walk would declare above it and print 10000 from. The connection's
half of the promise is line 3 instead: the mid-walk `query` is refused with 9,999 rows still on the
server, and a driver that had buffered the result set would have finished its statement and answered
it. No new `Core` member was invented and the check's `want` is untouched; only the comment above
the `[[check]]` moved, in `docs/agent/loop-goal.toml` and its byte-identical
`docs/agent/goals/21-carried-gaps.toml`.

**A double release in the `stream` slice is fixed** — `crates/nvs-stdlib/src/db/stream.rs:233`. The
new `Core\Db\Stream` was built with the connection's block-name string straight out of
`bind::handle_of`, which borrows it, while `crate::instance::build` takes a reference over; the
object's release then freed the connection's own name a second time. Every stream program printed
the right answer and exited **127**, the smoke run in the previous handoff included. The playbook
bullet under *Writing Novis itself* is the general rule.

**`streamAs<T>` is what is left of spec § 18**, and `stream` is still PostgreSQL-only — the other
four drivers throw a `RuntimeError` naming `query`, which is `crates/nvs-stdlib/src/db/mod.rs`'s gap
5 and deliberate rather than unfinished.

Nothing is blocked on a decision.

## Next group

**`streamAs<T>` — `queryAs<T>`'s shape over the walk `stream` already opens.** One file set:
`crates/nvs-stdlib/src/db/registry.rs`, `crates/nvs-stdlib/src/db/stream.rs`,
`crates/nvs-stdlib/src/db/execute.rs`, `crates/nvs-stdlib/src/db/row.rs`,
`crates/nvs-types/src/intrinsics.rs`.

- [ ] **The two rows and the card** — a `streamAs` beside each `stream`, under one symbol per ADR
      0043 as `queryAs` is: `crates/nvs-stdlib/src/db/registry.rs:313` (Connection),
      `crates/nvs-stdlib/src/db/registry.rs:576` (Transaction), the card beside
      `crates/nvs-stdlib/src/db/registry.rs:1715`'s `QUERY_AS_DOC`. The return type is a second
      `CoreTy::Instance` and its element type is declared in `ITERABLES` the way `STREAM`'s is.
- [ ] **The body** — `crates/nvs-stdlib/src/db/execute.rs:1077` is how a `<T>` member reads its class
      out of argument 0 and the `array<...>` refusal out of argument 1, and
      `crates/nvs-stdlib/src/db/stream.rs:209` is the statement half to clone; the class travels in a
      fourth slot on the stream object rather than being re-derived per row.
- [ ] **Per-row hydration** — `crates/nvs-stdlib/src/db/row.rs:90` builds one row into the declared
      class and is what `advance()` calls instead of `build(&ROW, …)` at
      `crates/nvs-stdlib/src/db/stream.rs:177`. A refusal names the column, as `queryAs` does.
- [ ] **The checker's row** — `crates/nvs-types/src/intrinsics.rs:223` and `:254` are `queryAs`'s two
      entries, and `crates/nvs-types/src/expr/calls.rs:185` is the comment saying a `<T>` member
      *produces* its type rather than reading one off the receiver.

## Backlog

- `stream`/`streamAs` on the other four drivers — `crates/nvs-stdlib/src/db/mod.rs` gap 5.
- `[context] modules` has no selector for `crates/nvs-stdlib/src/instance.rs`, whose `build`/`slot`
  ownership contract is the whole of this session's fix; add it to `docs/agent/loop-goal.toml`.
- `.agent-tmp/probe1.nvs`, `probe2.nvs` and the `nvs.toml` beside them are this session's scratch.
