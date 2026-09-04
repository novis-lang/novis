# Handoff

## State

**Goal 2 of the parity program has just started; nothing of it has landed yet.** Goal 1 — `Core`'s pure
half — reached its whole acceptance list, and that list plus M4's is now this goal's Stage 1 floor.
Nothing here touches `nvs-stdlib`'s pure members, so a red check there is a real regression.

**`crates/nvs-host` does not exist yet.** Coroutines live only in the M0 spike,
`benches/abi-probe/src/lib.rs`, whose four invariants are green and have been since then: a helper
suspends with JIT frames live above it, repeated suspends leave the frames intact, a throw still
propagates inside a coroutine, and many coroutines can be created and driven. `abi-probe`'s `Ctx` at
`benches/abi-probe/src/lib.rs:100` calls itself "deliberately shaped like the real `Ctx` will be" — read
it before designing a new one. This goal moves that spike into a crate and gives it a reactor; it does
not re-litigate whether the spike works.

**The one rule every session of this goal holds:** the runtime is ours and it is not `async`, and
`nvs-host`'s socket implements plain `std::io::Read`/`Write` while parking its coroutine. `docs/plan/design.md`
§ *Thread-per-core, shared-nothing runtime* is its only home.

## Next group

**ADR 0106's containment rule, which is Stage 0.** It was accepted after M4 was reported done and it
amends ADR 0002: containment moves outward from the helper to the worker task. It goes first because a
scheduler written against the old boundary has its panic path wrong in the place hardest to find later.

One file set: `crates/nvs-runtime/src/abi.rs`, `crates/nvs-runtime/src/ctx/isolate.rs`,
`benches/abi-probe/tests/invariants.rs`.

- [ ] **The worker task is the containment boundary.** ADR 0106 §§ 1–3. `catch_unwind` wraps the worker
      task; the existing helper-level boundary at `crates/nvs-runtime/src/abi.rs:336` is the *inner* one
      and stays. Two rules travel with it: nothing on a teardown path may panic, and teardown stops
      recursing — a panic while unwinding a panic is the shape that reaches `abort()`, which
      `catch_unwind` does not contain.
- [ ] **Every depth and duration a request can drive is bounded on the engine's own stack.** ADR 0106
      § 4. ADR 0020 § 1's call-stack bound gains an engine-side counterpart, and a bound inside a *single*
      helper joins it. Cheap now; expensive once there are twenty helpers that can recurse, and it is
      what makes a memory cap mean anything in goal 3.

## Backlog

- Stage 2 is the keystone and its item 4 — the reactor and the parking stream — carries one of this
  goal's two pre-authorized ADR slots. Read the goal file's § *Standing decisions* before opening it; the
  number comes from `python tools/brief.py`, re-checked immediately before the file is created.
- `spawn script` has parsed since M1 (`nvs_syntax::ast::ExprKind::SpawnScript`,
  `crates/nvs-syntax/src/ast.rs:931`) and lowers to nothing. The task forms have no grammar yet.
- ADR 0018's `TRACE`/`PROFILE` safepoint bits have no consumer and the three spawn-construct trace events
  wait with them. Off path — m5.md already says so.
