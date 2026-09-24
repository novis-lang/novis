---
milestone: M5
---
# Loop goal 2 — the reactor, the scheduler, and isolates

Finish **M5** — [docs/plan/m5.md](../../plan/m5.md) is the scope and this file does not restate it. A
Novis program can **suspend**: on a socket, on a timer, on a child task, on another core's answer — and a
core that is waiting on one request keeps serving the others.

This is goal `concurrency` of the parity program ([goals/README.md](README.md)) and it is **the keystone**. Nothing
above it has a socket, a timer or a task without it: the four database drivers, the outbound HTTP client
and the listener all reach the network through the one stream this goal builds. Every goal after this is
written against its shape, so a shape that is wrong here is wrong four times.

## The one design decision every session must hold

**The runtime is ours and it is not `async`.** `corosensei` stackful coroutines on a thread-per-core
scheduler, and no crate of ours depends on `tokio` — where it appears at all it is compiled with
`sync` alone, a channel library and not a runtime (`rule:concurrency/one-scheduler`).
`docs/plan/design.md` § *Thread-per-core, shared-nothing runtime* is the one home for the rule and this
file does not restate it.

What follows from it is the shape of item 2, and it is worth stating plainly because everything else in
the program depends on getting it right: **`nvs-host`'s socket implements plain `std::io::Read` and
`Write`, and parks its coroutine rather than blocking its core.** A read that would block returns to the
reactor, the coroutine is resumed when the descriptor is ready, and the *caller* sees an ordinary blocking
`read`. That is what lets every synchronous Rust crate compose with no async at all — `rustls` streams
over it unmodified, and so do the wire codecs goal `database`'s drivers use. A design that instead exposes futures
would put a second concurrency model beside the coroutines, which is exactly what `rule:concurrency/one-scheduler` refuses.

**The proof it works already exists.** `benches/abi-probe`'s coroutine invariants are green and have been
since M0: a helper suspends with JIT frames live above it, repeated suspends leave the frames intact, a
throw still propagates inside a coroutine, and many coroutines can be created and driven. This goal moves
that spike into `nvs-host` and gives it a reactor; it does not re-litigate whether the spike works.

## Stage 0 — the catch-up, and it is the containment rule

`rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers` was accepted after M4
was reported done and it **amends `rule:errors/propagation`**: containment moves outward from the helper to the worker
task. Every mechanism this goal builds sits inside that boundary, so it goes first — a scheduler written
against the old boundary is a scheduler whose panic path is wrong, and it is wrong in the place that is
hardest to find later.

1. **The worker task is the containment boundary, not the helper.** `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers`, `rule:http-server/containment-does-not-end-at-the-helper` and `rule:http-server/no-path-reaches-abort`: `catch_unwind` wraps
   the worker task; nothing on a teardown path may panic; teardown stops recursing. `nvs_runtime`'s
   existing `catch_unwind` at the ABI is the inner boundary and stays — this is the outer one.
   `crates/nvs-runtime/src/abi.rs:336` is the helper-body macro that owns the inner rule.
2. **Every depth and duration a request can drive is bounded on the engine's own stack.** § 4. The
   call-stack bound in `rule:errors/on-limit` gains an engine-side counterpart, and a single helper gains one too.
   This is the item that makes a memory cap mean anything later, and it is cheap now and expensive after
   there are twenty helpers that can recurse.

## Stage 0b — the catch-up, and it is calling a `Core` member by name

`rule:core-api/shape-rules` R2 was amended after this goal opened:
every `Core` parameter is callable by the `$name` [01-core-library.md](../../spec/01-core-library.md)
writes, and the trailing options bag by `options`, under exactly the rules a user-declared method already
has (`rule:types/arrays`). It is catch-up for the same reason Stage 0 is —
every case written in the meantime is written positional-only around a surface that is about to exist —
and it comes *after* Stage 0 in this file because the run order inside the catch-up class is this file's
order, and the containment rule is the one everything sits inside.

**Nothing about any member changes.** No parameter is added, removed, reordered or renamed; the bag stays
the bag. The items add a name to slots that already exist, and a check that the names are the spec's.

28. **Every registry row names its parameters.** `pub names: &'static [&'static str]` on
    `nvs_stdlib::registry::CoreMethod`, one per positional slot in `params` order, never the `$`; the bag
    slot is not written per row — it is `options` uniformly, stated once on `CoreTy::Options`. The names
    are the spec's signature column, and a `-p nvs-stdlib` test parses that column and holds every row to
    it (`crates/nvs-stdlib/src/registry.rs`; the rows are the `params: &[` tables in
    `crates/nvs-stdlib/src/*.rs`). Filling 341 rows by hand is the wrong spend: a scratch script that
    reads the spec and emits one `nv splice --patch` is the shape, and the guard test is what reviews it.
    `ParamDoc::name` stays as the description's key, and the existing doc-consistency test asserts it
    equals the row's.
29. **A `Core` member and the two synthesized signatures resolve a `name:`.**
    `crates/nvs-types/src/core_lib.rs` reads the row's names into `MethodSig::param_names` (plus `options`
    when the last parameter is a bag); `error_lib.rs`'s `Throwable` constructor becomes
    `["message", "options"]`; `iter_lib.rs`'s bodiless members take the spec's. After that no producer
    writes `None`: `param_names` becomes a `Vec`, `named_slot`'s `else` arm in
    `crates/nvs-types/src/expr/args.rs` goes, and `E_NAMED_ARG_NO_PARAM_NAMES` (E0485) is retired the way
    `nvs-diagnostics` retires a code — grep `E0485` across `tests/`, `crates/*/tests` and `docs/` first.
    `crates/nvs-ir/src/lower/call.rs` reads `arg_slots` uniformly already; confirm with a scratch run
    rather than by reasoning that a reordered name, a skipped defaulted positional (`defaults_of`) and a
    name at `Str::format`'s variadic tail (`E_UNKNOWN_ARG_NAME`) all behave at a helper call. `nvs meta
    --json` emits `names` for every row.
30. **The cases.** One `core` case calling a static and an instance member by name out of order, a
    defaulted positional skipped and the bag passed as `options:`, evaluation order shown by a
    side-effecting argument as `tests/conformance/lang/a-named-argument-binds-by-name-and-a-spread-by-position.nvst`
    does; one `error` case constructing a `Throwable` by `message:` alone and with
    `options: {previous: …}`; one `reject` case where a misspelled name at a `Core` member and a name at a
    variadic tail are both `E_UNKNOWN_ARG_NAME`. The docs are already written — `rule:core-api/shape-rules` R2, `rule:core-api/reference-card`, the
    spec's *How to read an entry*, and the comments at each site above say "Stage 0b lands it"; landing it
    means rewriting those comments to the present tense, not adding to them.

## Stage 1 — the floor

Goals `core-depth` and M4's whole acceptance lists, inserted mechanically by `goal-switch.py`, **never traded.**
`Core`'s pure half is finished and this goal does not touch it; a red check there is a regression.

## Stage 2 — the keystone: `nvs-host`, the reactor, and the parking stream

**No task, no isolate and no `Core\Task` member is written until this stage is green.** Everything above
is a consumer of it.

3. **`crates/nvs-host` exists, and one core runs one scheduler.** A per-core thread pinned to a CPU, a
   run queue of coroutines, and a `Ctx` carrying the yielder — the shape `benches/abi-probe/src/lib.rs:100`
   already models and calls "deliberately shaped like the real `Ctx` will be". Value refcounts stay
   non-atomic because a heap is only ever touched by one thread, which is `design.md`'s decisive
   structural choice and is what this crate must not break.
4. **The reactor, and the parking stream.** Readiness for a descriptor, per platform, and a `NvsTcp` (and
   its Unix-socket sibling) implementing `std::io::Read`/`Write` over it. **This is the item the whole
   program rests on** — see § *Standing decisions* for its ADR slot, which is the first slice of this
   stage rather than a follow-up to it.
5. **Timers.** A timer wheel on the same reactor, because a deadline is what `rule:concurrency/limit-and-deadline-are-the-only-bounds`'s
   `{limit, deadline}` and `rule:http-server/no-spelling-for-an-unbounded-wait`'s "no spelling for an unbounded wait" both resolve to. One
   implementation; a sleep and a deadline are the same mechanism seen twice.
6. **The blocking pool.** `rule:http-server/a-core-is-never-blocked-on-a-syscall`: filesystem calls, name resolution and waiting on a child process
   go to a pool **bounded at twice the core count**, because they have no readiness to wait on. The bound
   is per worker and never grows with requests served — that is the ADR's own footprint statement and it
   is not a number to tune during the run.
7. **The watchdog.** § 7: a thread reading the in-flight deadline each worker already maintains, reporting
   a worker whose oldest request has passed it by a configured margin. It costs the hot path nothing
   because it reads state the deadline mechanism keeps anyway — a heartbeat written per request is the
   wrong implementation and the reason this item names the mechanism.

## Stage 3 — `spawn`, `await`, and the task tree

8. **`spawn` and `await` lower.** The grammar has `spawn script` since M1
   (`nvs_syntax::ast::ExprKind::SpawnScript` at [ast.rs:931](../../../crates/nvs-syntax/src/ast.rs)); the
   task forms and their lowering are this item. A spawned task is a coroutine on the current core's queue.
9. **Structured concurrency: a task tree dies with its parent.** No orphans, and **no call returns with a
   child still running** — `rule:concurrency/nothing-is-still-running-when-a-call-returns`, which is the promise the rest of the roster is built on.
10. **Cancellation runs no user code.** § 5, and it is the item most likely to be got wrong in the
    obliging direction: native teardown runs, a cancelled task's `catch` and cleanup blocks **do not**, and
    its arena is released. The guard is a case asserting exactly that, because the intuitive
    implementation is the wrong one.
11. **`Core\Task\Channel`, with backpressure.** A bounded channel whose send suspends. Same file set as
    items 8–10.

## Stage 4 — the `Core\Task` roster

12. **`Core\Task::all` over a shape literal of `fn` literals**, each field keeping its own type — `rule:concurrency/all-answers-a-typed-shape`
    . A field holding a `callable` *variable* rather than an `fn` literal is a **compile error**, which
    is what makes the heterogeneous typing possible at all and is easy to leave out.
13. **`Core\Task::map`**, subject-first, which is what `parallel_map` became — § 2.
14. **`{limit, deadline}` is the one options shape**, in place of a `timeout` wrapper — § 3. `race` is
    deferred with a named future spelling and is **not** in scope.
15. **`Core\Task::afterResponse` and the `[deferred]` block** — §§ 6–7. The connection ends, the request
    tree does not; and what happens when the deferred queue is full is a decision that ADR already took.
    The `[deferred]` config block's *validation* is goal `governance`'s, since the registry does not exist yet; what
    lands here is the member and its behaviour under compiled-in defaults.

## Stage 5 — the graph copy, and both things carried by it

16. **One graph copy, two carriers.** `rule:classes/graph-copy`: the deep-copy-or-move walk is written once and reached twice — as the value-crossing operation at
    a `spawn worker`/`spawn script` boundary, and as `Core\Serialize`. Moving when the refcount is 1 is
    not an optimisation here, it is the semantics.
17. **`Core\Serialize::encode`/`decode`** — § 3, Novis's own closed byte format, versioned and
    self-describing, with **no** `__serialize`/`__wakeup`/`__sleep` hook. `decode` is a **`tainted` sink**;
    [01-core-library.md](../../spec/01-core-library.md) § 13 holds the reasoning and it is not restated in
    the implementation.
18. **A `secret`-qualified value is refused at the boundary** unless it went through
    `Core\Secret::reveal()` — `rule:security/secret-qualifier`.
    Same walk, one check.
19. **Bytes that are not Novis's own format are refused rather than partially accepted**, and so are bytes
    naming a class whose declared properties no longer match. A partially-accepted graph is the type
    confusion the sink exists to prevent.

## Stage 6 — `spawn script`, and the isolate

20. **The `Isolate` type in `nvs-host`**, with its own arena, `Core` accessor backing state and config
    overlay — `rule:security/isolate-shares-nothing`. It belongs in this goal rather
    than later because an isolate is a task with a heap boundary, which is exactly what Stage 2 built.
21. **The request tree and its shared budget**, § *Budgets are accounted at the root of the request tree*.
    Enforcement of the *limits* is goal `governance`'s; the accounting is this item's, and until then it runs under
    compiled-in defaults.
22. **`Core\Script::args()`, the top-level `return` contract, and the `ScriptResult` shape** — § *Failure
    is a value, not an exception*, plus `rule:statements/no-host-populated-variables`. A child's uncaught
    throw, its limit breach and a contained panic inside it all leave the parent running with `ok = false`.
23. **The value-crossing refusals**: a closure, a reference and a resource are refused at the boundary, and
    so is an unresolvable class. A cyclic argument crosses **without hanging**, which is the case that
    catches a naive walk.
24. **`output: capture|inherit`, `on: worker`, and cancellation of a child at its next safepoint.**
    Safepoints are emitted already and have been since the first backend commit; this is the first consumer.

## Stage 7 — the testing surface the isolate unlocks

25. **Every `#[Test]` runs in its own isolate**, sharing nothing but compiled code, with the runner owning
    the test's task tree — `rule:testing/isolate-per-test`, `rule:testing/determinism-declared-on-the-test` and `rule:testing/task-tree-and-virtual-clock`. This
    is what makes `nvs test` parallel, and it is here rather than at M4 because the isolate is here.
26. **`#[Test(at:, seed:)]` puts the clock and the generator under the test's control.** Same ADR, same
    sections. A test that is flaky because it read the wall clock is a test the language should have made
    impossible.

## Stage 8 — the numbers

27. **`benches/isolation.rs`, with the guard beside it.** Spawn-to-result for a trivial child on a warm
    cache is **single-digit microseconds**, committed next to the process baseline it replaces, and it
    sits alongside `an_os_process_costs_orders_of_magnitude_more_than_a_task` — which already exists and
    must stay green. m5.md's *Verify* paragraph is the authority on the rest: 100k concurrent tasks, a
    deliberate deadlock proving cancellation works, `Core\Task::map` near-linear across cores,
    ThreadSanitizer clean.

## Acceptance

**This goal is retired: its checks are the floor stage of [the live goal](../loop-goal.toml)**, carried
there by the switch that left it and folded forward at every switch since.

## Standing decisions — pre-authorized, do not stop the loop for these

- **Decide and record; never `BLOCKED` for a design call.** Record it in the home AGENTS.md names — a
  paragraph in `docs/adr/README.md` § *Decisions taken at project start*, or the crate's own module doc.
- **Two ADR slots, and no others.** Each is the first slice of the stage that needs it:
  - **The reactor and the parking stream** (Stage 2, item 4). What readiness mechanism each platform
    uses, what the parking contract is, what a `WouldBlock` costs, and how a coroutine's stack is
    accounted. This is the design four goals are written against and it may not live in a module comment.
  - **The isolate heap boundary** (Stage 6, item 20). What an arena is, what it costs, and how a value
    crosses — the parts `rule:security/isolate-shares-nothing` specifies as behaviour rather than as implementation.

  Anything else is decided-and-recorded. Claim the next free ADR number by creating the file, and
  **re-check it immediately before you do**: `python tools/brief.py` derives it from the directory.
- **Stage 0b opens no ADR and changes no member.** `rule:core-api/shape-rules` R2 already carries the rule; the names live
  on the registry row (`CoreMethod::names`), the bag is `options`, and the guard is the spec's signature
  column. Where a row's arity disagrees with the spec, the spec is authoritative for the *names* and the
  registry for what is *built*: give the row the spec's names and leave its `params` alone, and put a
  member whose shape the two genuinely disagree on in the handoff's Backlog — never a shape change.
- **No `tokio`, and this is not a judgement call.** If a capability appears to require an async runtime,
  that is a real `BLOCKED` naming the capability. Every other crate choice is yours under `rule:packaging/a-c-dependency-answers-two-questions`.
- **A blocking-looking read parks; it never blocks the core.** If a syscall has no readiness to wait on,
  it goes to item 6's pool. There is no third option, and "just this once" is how a core wedges.
- **Cancellation runs no user code**, and this is not softened when a fixture looks like it wants a
  `finally` to run. `rule:concurrency/cancellation-runs-no-user-code` decided it; the abandoned-generator rule M4 landed is a *different*
  mechanism about a program that suspended itself, and the two are not unified.
- **`race` does not exist**, and neither does a `timeout` wrapper. `{limit, deadline}` is the spelling.
- **`Core\Http\Client` is not in this goal.** The transport it will use is — a TCP stream and `rustls`
  over it — and the member surface, its address policy and its retry rules are goal `core-part-ii`'s.
- **The `[deferred]`, `[limits]` and `script.spawn` enforcement is goal `governance`'s.** This goal runs under
  compiled-in defaults and says so at each site, exactly as m5.md already does.

## What this goal does not touch

`Core`'s pure half (goal `core-depth`, and it is the floor) — except that Stage 0b adds a name to every slot it
already has, and changes no member's shape. Every capability-bearing `Core` member (goal `core-part-ii`) — the
transport is not the client. The listener (goal `server`). `rule:testing/debug-probes`'s `TRACE`/`PROFILE` safepoint bits, which
have no consumer until an exporter exists; the three spawn-construct trace events wait with them, and
m5.md already says so.
