# Handoff

## State

**Goal 2 of the parity program. The acceptance check's `0 containment` stage passes**: all three
tests it names are on disk in `benches/abi-probe/tests/invariants.rs`. The § 2 one existed under
another name and was renamed to the one the check asks for (playbook, *Tooling*); § 3's
teardown-under-unwind and § 4's engine-depth-on-a-coroutine-stack are new.

**ADR 0106 § 5's first half has landed.** `Ctx`'s hot line now carries a deadline flag at offset 16
with `DEADLINE_OFFSET` beside its neighbours, read by `Ctx::deadline_expired` and set through a
*shared* borrow by `Ctx::expire_deadline`. Why it is an `AtomicU64` of its own rather than another
`SafepointFlags` bit — the writer is a timer that by construction is not the thread inside the
helper — is `crates/nvs-runtime/src/ctx.rs`'s module doc § *The request's deadline*, which is that
decision's only home. **Nothing polls it yet**: the combinator and the cost bound are the next two
slices.

The outer containment boundary is unchanged: `nvs_runtime::run_task`, one `catch_unwind` per task,
with `TaskRoot::{Request, Worker}` carrying § 2's split. Teardown stays iterative and infallible —
`crates/nvs-runtime/src/release.rs`'s module doc is that rule's home.

**`crates/nvs-host` still does not exist**, so `orient.py` warns that the `[context] modules`
pattern `crates/nvs-host/src/*.rs` matches nothing. Expected until stage 2, not a manifest bug.
The four goal fixtures (`examples/tasks.nvs`, `channel.nvs`, `serialize.nvs`, `isolate.nvs`) are
untouched this session and still stop at their own item's first missing member.

**The one rule every session of this goal holds:** the runtime is ours and it is not `async`, and
`nvs-host`'s socket implements plain `std::io::Read`/`Write` while parking its coroutine.
`docs/plan/design.md` § *Thread-per-core, shared-nothing runtime* is its only home.

## Next group

**ADR 0106 § 5's other two halves.** The flag exists; nothing supplies the poll and nothing prices
it. One file set: `crates/nvs-runtime/src/ctx.rs`, `crates/nvs-runtime/src/abi.rs`,
`benches/abi-probe/tests/perf_guards.rs`, plus the one `Core` member that adopts it first.

- [ ] **A bounded-loop combinator supplies the poll**, rather than every helper remembering to.
      ADR 0106 § 5, first constraint — same reasoning as `nvs_helper!`, so the shape carries the
      obligation. `crates/nvs-runtime/src/ctx.rs:840` (`deadline_expired`), `:850`
      (`expire_deadline`), `crates/nvs-runtime/src/abi.rs:348` (`nvs_helper!`, the precedent), and
      `abi.rs`'s `Fault` for what a fired poll returns.
- [ ] **The cost bound the ADR owns**: the amortised poll stays under the stack check's own
      per-call cost. ADR 0106 § 5. `benches/abi-probe/tests/perf_guards.rs:181`
      (`an_all_bits_off_debug_probe_stays_in_the_safepoint_cost_class`) is the shape to copy —
      `#[cfg_attr(debug_assertions, ignore)]`, a `MAX_NS` const, and a threshold stated against
      `a_checked_return_frame_stays_cheap` rather than against a bare number.
- [ ] **One real O(input) `Core` member adopts the combinator**, with a case that the poll fires at
      a point where abandoning leaves the value consistent (§ 5's second constraint). Pick from
      `crates/nvs-stdlib/src/` — a scan or an encode, not a sort, for exactly that reason.

## Backlog

- The deadline flag's **writer** belongs to stage 2's reactor-and-parking ADR — goal standing
  decisions, ADR slot 1.
- `crates/nvs-host` does not exist; the `[context] modules` pattern matches nothing until stage 2
  (`docs/agent/loop-goal.toml`).
- The four goal fixtures stop at their own item's first missing member; each is rewritten by the
  session that lands that item (`docs/agent/goals/`).
- `Core\Task::all`/`::map`'s surface and its cancellation rules — ADR 0072 §§ 1-5.
- The isolate heap boundary ADR — goal standing decisions, ADR slot 2 (stage 6, item 20).
- M4 residue: the 1000-case conformance corpus count (`docs/implementation-plan.md`).
