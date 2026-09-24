# Handoff

## State

**Goal 173 — every resource FATAL reaches the program's onLimit handler — has just started; nothing of it has landed yet.** Goal `the-description-is-owed`'s whole list is this goal's Stage 1 floor.

`rule:errors/on-limit` already decides the behaviour: the handler fires for every resource-limit
`FATAL`, and it runs on a reserve carved out of the request's budget at request start. Nothing about
that is a session's to re-decide. What is open is one seam: a breach raised by
`crate::abi::affordable` **inside a member's own loop** stops the request and prints the ceiling
without running the handler, while the same breach raised by an array append in compiled code runs
it. `crates/nvs-runtime/src/sequence.rs`'s `# Known gaps` item 1 is the statement of the bug, and
`tests/conformance/core/arr-from-over-a-sequence-with-no-end-is-stopped-by-the-memory-ceiling.nvst`
pins the stop while asserting nothing about the handler.

## Next group

**Stage 2: the handler runs** — one file set: `crates/nvs-runtime/src/abi.rs`,
`crates/nvs-runtime/src/ctx/hooks.rs`, `crates/nvs-runtime/src/sequence.rs`,
`tests/conformance/core/`.

- [ ] **Find where a breach raised inside a member body is reported** — `crates/nvs-runtime/src/abi.rs:507`
      is the arm that asks `ctx.memory_breach()` and answered `None` for this shape; say what
      reports the ceiling instead, and whether the safepoint flag the refusal raised is what does it.
- [ ] **Make that path run the handler** — `crates/nvs-runtime/src/abi.rs:551`, the two lines
      `report_memory_breach` takes, are the shape to reach.
- [ ] **Pin it** — `tests/conformance/core/arr-from-over-a-sequence-with-no-end-runs-the-on-limit-handler.nvst`,
      the case this goal's Stage 2 check names, registering `Core\Fatal::onLimit` and driving
      `Core\Arr::from` over a sequence with no end.

## Backlog

- Stage 0's catch-up is one deletion: `crates/nvs-runtime/src/sequence.rs`'s `# Known gaps` item 1,
  removed by the session that closes this goal.
- When this goal's last check goes green the driver takes goal `plain-comments`.
