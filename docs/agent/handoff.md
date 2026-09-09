# Handoff

## State

**Goal 39 is closed.** Stage 3 landed the log target's window and stage 4 flipped the rulebook, so
both of the goal's rules are `shipped`: `rule:errors/a-record-names-where-it-was-produced` and
`rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`.

`Core\Log::write` coalesces through `crates/nvs-runtime/src/floor.rs`'s fixed table —
`LOG_WINDOW_SLOTS` slots, scanned rather than indexed so two records can never collide into one,
evicting a free slot first and then the window nearest to closing. The floor keeps its single slot
and `COALESCING_WINDOW` unchanged, and both sinks share one `key`, so what counts as the same record
is decided once for both. Nothing touches the debug stream ([0165](../decisions/0165.md) § 3).

The coalescing is visible to any test that writes one record twice, which is what the playbook
bullet is for: two `-p nvs-stdlib` envelope-shape tests wrote one message across three contexts and
now give each write its own message.

## Next group

**Stage 3 follow-on: the member's own coverage, which the six unit tests reach only through
`nvs-runtime`** — one file set: `crates/nvs-stdlib/src/log.rs`, `tests/conformance/core/`. A goal
switch overwrites this file, so this group holds only if the driver stays on goal 39.

- [ ] **A `-p nvs-stdlib` test drives `Core\Log::write` through the table** —
      `crates/nvs-stdlib/src/log.rs:572` is `written`, the helper that calls the member for real, and
      `crates/nvs-runtime/src/floor.rs:407` is `expire_log_windows`. Assert that the second identical
      write buffers nothing and that the next one after the window closes carries `count`, which no
      test asserts of the member itself today.
      `rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`.
- [ ] **A conformance case for one line where a program wrote two** —
      `tests/conformance/core/log-write-renders-one-json-line-per-record.nvst:9` is the shape to
      write beside: one message written twice inside the window is one line, and a message differing
      only in its own `source` line is a second one.
      `rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`.

## Backlog

- Read-time grouping for the debug stream is [0163](../decisions/0163.md)'s ingester and viewer, and
  deliberately not this bound — `docs/decisions/0165.md` § 3.
- A per-call-site rate limit is the only thing that would make a hot loop cheap rather than quiet,
  and nothing has measured for it — `docs/agent/loop-goal.md` § *Standing decisions*.
- `[log] format` is still unread at run time, so JSON Lines is the single answer —
  `crates/nvs-runtime/src/floor.rs`'s module doc.
