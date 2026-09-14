# Handoff

## State

**Goal `m5-proofs` (M5). Stage 4 is closed, and stage 3's two `.nvst` cases are the earliest red check.**

Stage 4's isolate half is on disk. `Completion` (`crates/nvs-runtime/src/host.rs:268`) carries `wall`, the
child's own body time, read only where the parent opened an event and stopped where the body stops — so
the answer's crossing lands on the parent's side of the overhead split rather than inside the child's
compute. `Isolate::start` (`crates/nvs-host/src/isolate.rs:423`) opens one `TraceKind::Spawn` event per
`spawn script`, both `Running` handles close it at the join with that number, and a child that is
abandoned rather than joined leaves its event open. All four of the stage's test names now resolve.

One thing the rule says that the tree does not: `rule:observability/spawn-is-its-own-event` describes the
child's wall time as "already arriving on `ScriptResult`", and that shape's four fields
(`crates/nvs-stdlib/src/script.rs:428`) do not carry it — the split reads it off the native `Completion`
before the language shape is built. Backlog.

Nothing is blocked.

## Next group

**Stage 3: the isolate proofs** — one file set: `docs/agent/loop-goal.toml`,
`docs/agent/goals/55-m5-proofs.toml` and `tests/conformance/core/`.

- [ ] **The drafted isolate case is on disk under the name its author chose** — the check at
      `docs/agent/loop-goal.toml:9759` names
      `tests/conformance/isolate/a-child-reading-request-or-session-state-throws-a-logic-error.nvst`,
      and `tests/conformance/isolate/a-child-refuses-request-state-and-still-reads-the-process.nvst:2`
      is that claim whole, `Core\Server::isDraining` included. Re-point the name in both toml copies,
      which are byte-identical by construction. `rule:security/request-state-throws-in-an-isolate`.
- [ ] **A session opened where no request arrived is a `LogicError`** — write
      `tests/conformance/core/session-start-where-no-request-arrived-is-a-logic-error.nvst`, the check's
      second case: a program answering no request calling `Core\Session::start()`, whose refusal
      `crates/nvs-stdlib/src/session.rs:284` spells and `crates/nvs-stdlib/src/session.rs:735`
      documents. `tests/conformance/core/session-start-refuses-a-tree-that-configured-no-store.nvst:1`
      is the nearest shape to copy, and it answers a request where this one must not.
      `rule:security/request-state-throws-in-an-isolate`.
- [ ] **Stage 3's other check is already green, so confirm rather than write** — the three names under
      `docs/agent/loop-goal.toml:9767` are in `crates/nvs-host/src/isolate.rs`; a `cargo test -p
      nvs-host` filter says so in one call, and a miss there is the whole of that stage's remaining
      work. `rule:security/isolate-shares-nothing`.

## Backlog

- `ScriptResult` carries no child wall time, against `rule:observability/spawn-is-its-own-event`'s own
  sentence — either a fifth member in `crates/nvs-stdlib/src/script.rs:428` or a corrected fragment.
- Stage 5: the record, and the concurrency rule its check names by id — the ruling it writes down is
  already settled in `docs/agent/loop-goal.md` § *Standing decisions*.
- Stage 6: `on:`, `limits:` and `grants:` — two `-p nvs-types` tests, three `-p nvs-host` tests and
  three `.nvst` cases, listed in `docs/agent/loop-goal.toml`'s `stage = "6 the options"`.
- A group with no task beneath it (`run_here`) records no spawn event, because nothing is started
  there — `crates/nvs-host/src/group.rs`'s module doc if it ever needs saying.
