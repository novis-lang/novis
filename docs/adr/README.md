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

`.claude/brief.py` needs no update for any of this: it slices this file's table and the plan's status block
live, and its budgets are derived from the row count, so a new row is picked up automatically and can never
put it over. The one rule a new row must meet is that its **Decision** cell stays one sentence under 160
bytes, and that any `|` inside it is written `\|`; `python .claude/brief.py --check` reports both.

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
| [0015](0015-no-name-aliasing.md) | No `class_alias` or import `as`; `type` aliases are the disciplined exception | Accepted |
| [0016](0016-ide-integration.md) | IDE integration is a thin per-editor client over one language server; PhpStorm goes LSP-bridge before native | Accepted |
| [0017](0017-hot-reload-without-restart.md) | The compiled-unit cache revalidates lazily and swaps a per-path pointer; no filesystem watcher, no restart | Accepted |
| [0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md) | Coverage, tracing and profiling are safepoint-shaped probes, not a second compiled tier | Accepted |
| [0019](0019-reflection-and-ast-parsing-are-core-features.md) | Reflection and AST/source parsing are first-class `Core` features, not aftermarket extensions | Accepted |
| [0020](0020-error-escalation-ladder.md) | Fatal errors escalate through a reserved-budget handler ladder, never through `catch` | Accepted |
| [0021](0021-single-file-inclusion-construct.md) | `require` is the only same-frame file-inclusion construct; `include`/`include_once`/`require_once` are rejected | Accepted |
| [0022](0022-definite-property-initialization.md) | Properties are definitely initialized at compile time; no new `undefined` type, no silent defaults | Accepted |
| [0023](0023-clone-serialize-and-cross-boundary-copy.md) | `clone` stays PHP-shallow; `serialize`/`unserialize` share one graph-copy operation with the isolate boundary; neither is hookable | Accepted |
| [0024](0024-taint-tracking-for-injection-sinks.md) | Untrusted input is a distinct type; injection sinks demand laundering | Accepted |
| [0025](0025-wasm-browser-target.md) | The browser is a second compile target, not a second language | Accepted |
| [0026](0026-performance-measurement-methodology.md) | Performance history is tracked by callgrind instruction counts; wall-clock stays for CI regression guards | Accepted |
| [0027](0027-callable-is-closures-only.md) | `callable` means a `Closure`; PHP's string/array callable spellings and `__invoke` are both rejected | Accepted |
| [0028](0028-closing-the-remaining-magic-methods.md) | `Stringable` replaces `__toString`; no `__destruct`, `__debugInfo`, or `__set_state`; `unset()` is refused on an object property | Accepted |
| [0029](0029-identifier-casing-is-checked.md) | Identifier casing is a hard compiler error — `PascalCase` types, `camelCase` members, `SCREAMING_SNAKE_CASE` constants, no suppression | Accepted |
| [0030](0030-no-leading-underscores-constructor-spelling.md) | No leading underscores anywhere; the constructor is spelled `constructor`, not `__construct` | Accepted |
| [0031](0031-callable-is-the-only-closure-type.md) | `fn` is the only closure literal, with no `use` clause; `callable` absorbs `Closure` as the one surviving type name | Accepted |
| [0032](0032-acronym-casing-rule-revoked.md) | The acronym-as-one-word casing rule is revoked; only an identifier's leading character is checked | Accepted |
| [0033](0033-secret-qualifier-for-confidential-values.md) | `secret` is a second compile-time qualifier alongside `tainted`; HTML output, `Core\Log`, debug dumps, exception messages and serialize all refuse it by default | Accepted |
| [0034](0034-legacy-cast-syntax-rejected.md) | PHP's legacy `(T)expr` cast syntax is rejected; `as` is the only conversion spelling | Accepted |
| [0035](0035-truthy-boolean-context.md) | A condition is judged by PHP's full truthy table; every other `bool` position stays checked | Accepted |
| [0036](0036-anonymous-object-shapes.md) | `object` is the opaque top of every class type; `{...}` builds an anonymous methodless instance, and `{name: T, ...}` is MWL's one structurally-checked type | Accepted |
| [0037](0037-var-local-type-inference.md) | `var $name = expr;` infers a local's type from its initializer and fixes it forever; a bare array-literal initializer is the one shape it refuses | Accepted |
| [0038](0038-lateinit-property-modifier.md) | `lateinit` defers a non-nullable object property's first assignment past the constructor, throwing on read-before-write; `?T` and `readonly` are refused | Accepted |
| [0039](0039-canonical-code-formatting.md) | `mwl fmt` is one canonical, unconfigurable, PER-based formatting style with no reflow; it is never wired into the compiler | Accepted |
| [0040](0040-vscode-deep-tooling-and-resilient-parsing.md) | The VS Code extension goes deep (inspections, refactorings, Test Explorer, debugger UI) ahead of M10; `mwl-syntax` gains a resilient parse mode | Accepted |
| [0041](0041-timeline-export-and-gc-spawn-trace-events.md) | Trace events gain a `call`/`gc`/`spawn` kind and a speedscope-evented export, so a request's timeline shows GC pauses and isolate boundaries, not just calls | Accepted |
| [0042](0042-on-disk-artifact-cache-format.md) | The on-disk artifact cache is one immutable, self-describing file per compiled unit, verified before it is ever mapped executable | Accepted |
| [0043](0043-interface-default-methods-and-delegation-replace-traits.md) | There is no `trait`; interface default/private methods share behavior and explicit `by` delegation shares state, with one conflict rule and no `insteadof` | Accepted |
| [0044](0044-core-process-argv-only-no-shell.md) | `Core\Process` is the one argv-only way to run another program; there is no shell-string form, and a Windows batch/PowerShell target is refused outright | Accepted |
| [0045](0045-and-or-xor-keyword-operators-rejected.md) | PHP's `and`/`or`/`xor` keyword operators are rejected; `&&`/`\|\|` are the only logical connectives | Accepted |
| [0046](0046-attributes-shape-literal-metadata.md) | `#[...]` attributes are shape-literal metadata, checked structurally, retrieved via `Core\Attributes::get<T>`/`::all<T>` | Accepted |
| [0047](0047-literal-and-enum-case-types.md) | A scalar literal or a named enum case is itself a type; unioning them declares an explicit closed set, checked like any other conversion | Accepted |
| [0048](0048-portable-single-file-executables.md) | A portable single-file executable appends source to the host binary; rebundling is a build-time CLI step, not a runtime one | Accepted |
| [0049](0049-single-open-tag-and-single-exit-keyword.md) | `<?php` and `die` are rejected; `<?mwl` and `exit` are the only spellings kept | Accepted |
| [0050](0050-list-destructuring-spelling-rejected.md) | `list(...)` is rejected; `[...]` is the only destructuring spelling | Accepted |
| [0051](0051-standard-library-tiers.md) | Six ordered tests place every stdlib candidate at Core, Native, Ext, dropped, or already-answered; PHP's extension partition is not inherited | Accepted |
| [0052](0052-closed-doors.md) | Four closed doors: no FFI, no stream wrappers, no cross-request state, no `eval` | Accepted |
| [0053](0053-iteration-and-generators.md) | `Iterable`/`Iterator` are the only iteration interfaces; generators exist and lower to state machines | Accepted |
| [0054](0054-decimal-scalar-type.md) | `decimal` is a scalar type; `bcmath` and `gmp` are retired | Accepted |
| [0055](0055-extension-qualifier-declarations.md) | Extension manifests carry `tainted`/`secret`; every declaration tightens, none loosens | Accepted |
| [0056](0056-regex-engine-policy.md) | Regex runs on a linear-time engine by default; backtracking is opt-in and budgeted | Accepted |
| [0057](0057-intrinsic-literal-folding.md) | Literal arguments to a closed list of intrinsic `Core` calls are validated and prepared at compile time | Accepted |
| [0058](0058-outbound-request-policy.md) | Outbound connections carry an address policy; a tainted URL must be laundered and pinned | Accepted |
| [0059](0059-cross-request-state-is-explicit.md) | `Core\Cache` is per-core, copied in and out, and charged to the core rather than a request | Accepted |
| [0060](0060-application-security-protocols.md) | A closed roster of application-layer security protocols lives in `Core` | Accepted |
| [0061](0061-compile-time-autoload-and-program-discovery.md) | `autoload` maps names to files at compile time; `Core\Program::implementing<T>()` enumerates classes nothing names | Accepted |
| [0062](0062-case-sensitivity-is-a-compiler-property.md) | Names resolve case-sensitively, reserved spellings are lower case only, and a `require`/`autoload` path must match the on-disk entry exactly | Accepted |

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
in [0006](0006-isolated-script-execution.md), deliberately: one set of value-crossing rules, not two — and
[0023](0023-clone-serialize-and-cross-boundary-copy.md) gives that one rule its formal definition, shared
with `serialize()`/`unserialize()`.

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
