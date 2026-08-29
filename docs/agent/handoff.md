# Handoff

## State

**Goal 2 of the parity program. ADR 0106 § 2's containment rule has landed**, and the outer boundary is
`nvs_runtime::run_task` in `crates/nvs-runtime/src/abi.rs` — one `catch_unwind` per *task*, applied by
whatever spawns it, with `TaskRoot::{Request, Worker}` carrying § 2's split and `TaskPanic::retires_worker`
answering it. The `nvs_helper!` wrapper is untouched and is the **inner** boundary; a fault it catches
becomes `FATAL` on the `Ctx` and never reaches the outer one. Teardown stays iterative and infallible —
`crates/nvs-runtime/src/release.rs`'s module doc is that rule's home.

**`crates/nvs-host` still does not exist**, so `orient.py` warns every session that the `[context] modules`
pattern `crates/nvs-host/src/*.rs` matches nothing. That is expected until stage 2, not a manifest bug.
Coroutines live only in the M0 spike, `benches/abi-probe/src/lib.rs`.

**The acceptance check runs its floor again.** All 35 fixtures `loop-goal.toml` names are on disk: the four
this goal adds — `examples/tasks.nvs`, `channel.nvs`, `serialize.nvs`, `isolate.nvs`, plus three children
under `examples/isolate/` — were missing, and `LoopGoal.begin` aborts the *whole* check before the first
build on the first one of those (the playbook bullet at *Tooling* owns this; it is the second goal switch
it has cost). Each of the four now compiles as far as its own item and stops there: `Core\Task has no
member named all`, `Core\Task\Channel is not declared`, `Core\Serialize has no member named encode`,
`spawn script is not compiled yet`. Their **source is not frozen and their four output lines are**, so the
session that lands an item rewrites the block that reaches it.

**The one rule every session of this goal holds:** the runtime is ours and it is not `async`, and
`nvs-host`'s socket implements plain `std::io::Read`/`Write` while parking its coroutine.
`docs/plan/design.md` § *Thread-per-core, shared-nothing runtime* is its only home.

## Next group

**ADR 0106 § 5 — time is bounded *inside* a helper, not only between helpers.** The deadline flag goes in
the hot cache line the stack check already loads, so a poll is a compare rather than a load, and the poll
is supplied by a combinator rather than remembered per helper. Untouched by this session: the fixtures
above share no file with it.

One file set: `crates/nvs-runtime/src/ctx.rs`, `crates/nvs-runtime/src/abi.rs`,
`benches/abi-probe/tests/perf_guards.rs`.

- [ ] **The request's deadline flag lives in `Ctx`'s existing hot line.** ADR 0106 § 5. Beside
      `safepoint`/`stack_limit`, with an `offset_of!` constant like its neighbours —
      `crates/nvs-runtime/src/ctx.rs:576` (`SAFEPOINT_OFFSET`), `:583` (`STACK_LIMIT_OFFSET`), `:748`
      (`arm_stack_limit`, the one place a bound is computed), `:1439` (`nvs_stack_check`).
- [ ] **A bounded-loop combinator supplies the poll**, rather than every helper remembering to. ADR 0106
      § 5's first constraint — `crates/nvs-runtime/src/abi.rs`, next to `nvs_helper!`.
- [ ] **The cost bound the ADR owns**: the amortised poll stays under the stack check's own per-call cost,
      asserted in `benches/abi-probe/tests/perf_guards.rs`.

## Backlog

- `examples/serialize.nvs:71` holds a **placeholder** byte literal for item 19's shape-mismatch case. One
  program cannot produce those bytes (a file declares `Slot` once), so the session that lands ADR 0023
  § 3's format writes a real encoding over it as its last step.
- `examples/channel.nvs` *proposes* the `Core\Task\Channel` surface — `new Channel(2)`, `send`, `close`,
  `foreach` — because no ADR pins it (ADR 0072's scope line defers it to M5). Item 11's session owns the
  final spelling; what it may not change is that a full send suspends.
- `examples/isolate.nvs` writes its child paths from the repository root. If item 20 resolves a child
  relative to the entry file, those three lines shorten. ADR 0104 says nothing either way.
- ADR 0106 § 4's remaining half — a different file set from § 5's, which is why it is not in the group.
- `docs/adr/README.md` § *Where to look* has no row for the fixture-to-stage mapping; `loop-goal.toml`'s
  `stage` strings are the only home and that is fine.
