# Architecture Decision Records

Each ADR records a decision that would be expensive to reverse, and *why* — so that a future reader can
tell a deliberate trade-off from an accident. Add one whenever a choice constrains later work.

Files get their own document only when the reasoning is subtle or contested. Decisions that are simply
recorded, with no live tension worth arguing, live in *Decisions taken at project start* below.

**Reading one.** Every ADR opens with a metadata block and an **In short** paragraph carrying the whole
decision. If you only need the rule, stop there. `Context`, `Investigation` and `Alternatives rejected`
exist for when you intend to *change* the decision. Numbering starts at 0002; there is no 0001, and the
project-start section below is what would have been it.

**Measured numbers.** Each ADR quotes only its own measurements, and every one is guarded by a test in
[`benches/abi-probe`](../../benches/abi-probe/) named in that ADR's *Validated by* line. The tests are
authoritative; a number written anywhere else is a copy that can go stale.

**Adding a decision.** Touch exactly these, in order — nothing else should ever need its own copy:

1. Write the ADR file, following the shape every other one has: metadata block, **In short**, then
   `## Context` / `## Investigation` / `## Alternatives rejected` / `## Decision` / `## Consequences` as
   needed.
2. Add its row to the table below: number, one-line decision, status.
3. If it changes a prior ADR's decision, add **Amends** / **Amended by** lines linking the two, both
   directions.
4. If the topic is one an agent will search for by keyword, add one row to
   [CLAUDE.md](../../CLAUDE.md)'s "Where to look" table, and — only if it is a hard invariant — one bullet
   (a sentence, not a paragraph) to its "Ground rules enforced elsewhere".
5. If it changes what a milestone builds, update that milestone's paragraph in
   [the plan](../implementation-plan.md) with a link and a headline, not a restatement.

`.claude/brief.sh` needs no update for any of this: it slices this file's table and the plan's status block
live, so a new row is picked up automatically.

| # | Decision | Status |
|---|---|---|
| [0002](0002-error-propagation.md) | Exceptions propagate by checked return, not by unwinding | Accepted |
| [0003](0003-extension-system.md) | Extensions are sandboxed WebAssembly components, not native shared libraries | Accepted |
| [0004](0004-memory-for-simplicity.md) | Memory is spent for security, speed and simplicity, in that order | Accepted |
| [0005](0005-config-changeability.md) | `mwl.ini` states defaults, not ceilings | Accepted |
| [0006](0006-isolated-script-execution.md) | Running another script is an in-process isolate, not a subprocess | Accepted |
| [0007](0007-explicit-type-system.md) | Types are declared, checked, and never change by themselves | Accepted |
| [0008](0008-static-and-global.md) | `static` marks a class member; there are no function statics and no `global` | Accepted |
| [0009](0009-string-and-bytes.md) | `string` is text; binary data is a distinct `bytes` type | Proposed |
| [0010](0010-enums-are-a-value-type.md) | Enums are a closed, named integer type, not PHP's class-like construct | Accepted |
| [0011](0011-functions-and-constants-are-class-members.md) | Functions and constants are class members; `Core` is the reserved namespace for built-ins | Accepted |
| [0012](0012-no-superglobals.md) | There are no superglobals; request, session, environment and CLI state are `Core` accessor classes | Accepted |
| [0013](0013-comparable-interface.md) | Ordering two objects requires `Comparable`; PHP's property-walk fallback is rejected | Accepted |
| [0014](0014-property-observer.md) | Property hooks feed a declared `PropertyObserver`; no undefined-property fallback, no `__call`/`__callStatic` | Accepted |
| [0015](0015-no-name-aliasing.md) | No `class_alias`, import `as`, or trait-use `as`; `type` aliases are the disciplined exception | Accepted |
| [0016](0016-ide-integration.md) | IDE integration is a thin per-editor client over one language server; PhpStorm goes LSP-bridge before native | Accepted |
| [0017](0017-hot-reload-without-restart.md) | The compiled-unit cache revalidates lazily and swaps a per-path pointer; no filesystem watcher, no restart | Accepted |

## Decisions taken at project start

Recorded here rather than as individual ADRs. Promote one to its own file if it is ever seriously
challenged.

**Rust as the implementation language.** Memory safety in the runtime is a product requirement, not an
implementation preference; Cranelift, the async ecosystem and the pure-Rust protocol crates all live here.

**Cranelift JIT as the only execution tier, no interpreter.** Chosen for peak performance and a single
semantics implementation to keep correct. The cost is that the first runnable program requires the whole
front end plus a working backend. Mitigated by shipping a *baseline* tier where every operation lowers to a
call into a Rust runtime helper — mechanically close to an interpreter loop, therefore quick to get
correct — with typed inlining layered on later behind the same IR boundary.

**Thread-per-core, shared-nothing runtime.** One single-threaded executor pinned per core; a request is
assigned to a core and never migrates. This is what makes value refcounts *non-atomic* (a heap is only ever
touched by one thread), makes cross-request state contamination structurally impossible rather than merely
prevented, and still uses every core — parallelism comes from N independent executors. Compiled code is
immutable and therefore shared across all cores through `Arc` with no copying.

**Stackful coroutines for suspension.** Validated by spike #3, now the guard tests
`a_helper_can_suspend_with_jit_frames_live_above_it` and `a_coroutine_round_trip_stays_cheap` in
[`benches/abi-probe`](../../benches/abi-probe/), which hold the current figure for a suspend/resume round
trip through live JIT frames. The decisive property is the absence of *function colouring*: any MWL
function may perform I/O and yield without being marked `async`, so converted PHP call chains become
concurrent with no rewriting. The cost is a stack per in-flight task (default 64 KiB, configurable, grown lazily) and a small
audited unsafe core for stack switching, taken as a dependency (`corosensei`) rather than hand-rolled. The
memory is paid deliberately, under [0004](0004-memory-for-simplicity.md).

**Isolated workers for CPU parallelism.** Work dispatched to another core gets its own heap; values
crossing the boundary are deep-copied, or moved when the refcount is 1. Data races are impossible by
construction rather than by discipline, which is what lets the refcounts stay non-atomic. The copy is
another instance of [0004](0004-memory-for-simplicity.md). The same rules govern the script-level boundary
in [0006](0006-isolated-script-execution.md), deliberately: one set of value-crossing rules, not two.

**Strict shared-nothing requests.** Only compiled code survives a request. The consequence — reconnecting
to the database every request — is accepted for v1; `mwl-host` reserves an unused `PersistentRegistry` seam
so pooling can be added later without redesign. The same isolation is reachable from inside the language:
`spawn script` runs another `.mwl` file as a child isolate of the request tree
([0006](0006-isolated-script-execution.md)), and an inbound request is simply the root isolate of its tree,
so both paths are one implementation.

**Safepoints emitted from the first backend commit.** A poll at every loop back-edge and function entry is
the single mechanism behind CPU-time limits, client-disconnect cancellation, the cycle collector, the
profiler, debugger breakpoints and later deoptimisation. Retrofitting it would mean rewriting codegen, so
it is not deferrable.

**Server-level configuration, not per-project.** `mwl.ini` is root-owned, php.ini-style, and per-app
capability blocks live in the *root* config so an application can never grant itself rights. What a script
may change about its own configuration at runtime is per-directive and is argued in
[0005](0005-config-changeability.md).

**Pure-Rust dependencies by default.** A memory-safe runtime cannot contain arbitrary C. Deviations are
explicit, argued and few — currently only SQLite (`rusqlite`), where no credible pure-Rust implementation
exists.

**Checked-return call sites go through one code path.** See [0002](0002-error-propagation.md): a missing
status check would silently swallow an exception, so no caller constructs a raw `call` instruction.

**`unsafe` is confined to named crates, each declaring its own policy.** The workspace sets
`unsafe_code = "forbid"`; crates that genuinely need it opt down to `deny` and allow individual blocks with
a stated reason. Currently that is `mwl-runtime` and `mwl-codegen` (planned: the coroutine stack switcher,
the request arena, JIT page mapping) plus `benches/abi-probe`, which must call JIT-compiled code to
measure it. The probe is `publish = false` and is not a dependency of anything shipped, so it does not
widen the runtime's unsafe surface.

**Architecture assumptions are tested, not remembered.** Several decisions here rest on how Cranelift,
`corosensei` and Wasmtime behave rather than on our own code, and a dependency bump can invalidate them
silently. `benches/abi-probe/` checks them on every CI run, including the *premise* of
[0002](0002-error-propagation.md) — that native unwinding through JIT frames is unavailable — so if that
ever changes we are told rather than left paying for a workaround that is no longer needed. It also guards a
premise about the *platform we are replacing*: that an OS process costs orders of magnitude more than a task,
which is the whole cost argument for [0006](0006-isolated-script-execution.md).
