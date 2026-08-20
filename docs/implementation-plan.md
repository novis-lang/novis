# MWL — Modern Web Lang: Implementation Plan

> **Status — 2026-08-20.** Milestone **M0**, closing. Nothing runs yet; `Hello World` is M3.
>
> **On disk:** the workspace, CI across three platforms, the lint/deny/fmt policy,
> `crates/mwl-diagnostics`, and [`benches/abi-probe`](../benches/abi-probe/) holding the promoted
> M0 spikes as permanent guard tests. Every other crate in the layout is unwritten, and is created
> when its milestone starts rather than sitting empty.
>
> **Toolchain in place:** Rust 1.97.1 stable (pinned), Cranelift 0.128.4, wasmtime 41, MSVC 14.44
> + Windows SDK 10.0.26100 for linking, PHP 8.5.8 available as a comparison oracle.
>
> **Next, in order:**
>
> 1. `docs/spec/00-overview.md` — the semantics need writing down before code encodes them by
>    accident. It must define `spawn script` next to `include`, since the two run a file and
>    isolate opposite amounts, and that is the confusion
>    [ADR 0006](adr/0006-isolated-script-execution.md) predicts. It also owns the *spelling* of the
>    type surface whose semantics [ADR 0007](adr/0007-explicit-type-system.md) fixes — the
>    declaration slots, `array<T>`, and the conversion operator. Its scoping section states what
>    [ADR 0008](adr/0008-static-and-global.md) decided: which `static` survives, and that the list of
>    places state may outlive a call is closed.
> 2. The remaining ADRs for the decision table below. [0002](adr/0002-error-propagation.md)
>    through [0008](adr/0008-static-and-global.md) and [0010](adr/0010-enums-are-a-value-type.md)
>    through [0014](adr/0014-property-observer.md) are written and Accepted;
>    [0009](adr/0009-string-and-bytes.md) is drafted but Proposed, pending the cost measurement its own
>    *Revisiting* names.
> 3. Begin M1 with the lexer — inline-HTML mode plus interpolation shapes every layer above it.

**How this document relates to the ADRs.** This is the plan of record: *what* gets built, in what order,
and how each milestone is verified. It states decisions but does not argue them. The reasoning lives in
[docs/adr/](adr/README.md), and where a decision has its own ADR this document links to it instead of
restating it — follow the link rather than expecting the argument here. For decisions with no ADR of their
own, the *why* is in [adr/README.md](adr/README.md) under *Decisions taken at project start* and the
*mechanics* are in this document's **Architecture** section.

## Context

This is the living implementation plan for **MWL**, the language this repository builds. It is the
single place where the plan of record lives; every decision it states is argued out in full in
[docs/adr/](adr/README.md). Keep the status block above in sync as milestones land.

The goal is a new programming language for web servers and CLI, written in Rust, that:

- takes PHP 8.5 syntax as its starting point so existing PHP projects can be migrated,
- compiles to native code just-in-time with no build step (edit file → run),
- has first-class in-language parallelism,
- can run another `.mwl` file as a fully isolated unit of work **inside the same process**, so a script
  never has to spawn an interpreter to get isolation ([ADR 0006](adr/0006-isolated-script-execution.md)),
- is memory-safe and hard to attack,
- serves HTTP from a **single process** handling unlimited concurrent, fully isolated requests while
  sharing one compiled-code cache across all of them,
- and is fast, safe and simple *first* — spending memory to stay that way rather than the reverse
  ([ADR 0004](adr/0004-memory-for-simplicity.md)).

The motivation is the structural ceiling of PHP itself: process-per-request or worker-pool models, no
in-language parallelism, no way to run code in isolation short of another process, a C runtime with a long
CVE history, and a stdlib whose semantics block optimisation. MWL keeps PHP's authoring experience (no
build step, inline templating, familiar syntax) and replaces the execution model underneath it.

Intended outcome: a self-hosted toolchain (`mwl` binary) that runs `.mwl` files on the CLI, serves them
over HTTP from one process, and can mechanically transpile existing PHP codebases — including their
`.phpt` test suites — into MWL.

---

## Confirmed design decisions

| Area | Decision |
|---|---|
| Implementation language | Rust (stable, pinned via `rust-toolchain.toml`) |
| Resource priorities | **Security → semantics → latency → simplicity → memory footprint.** Memory is spent to buy the other four, within an enforced per-request cap. Not a low-footprint runtime |
| Execution | **Cranelift JIT from day one.** No interpreter tier. Baseline codegen first, optimising tier later |
| Code cache | Content-addressed on-disk cache (BLAKE3) + in-process `Arc` sharing |
| Parallelism | **Hybrid**: `async`/`await` for I/O inside a task (same heap, cooperative) + isolated workers on other cores for CPU work |
| Suspension | **Stackful coroutines** — no async colouring; any function may yield |
| Isolated execution | **`spawn script 'file.mwl'`** — runs another file in-process with its own heap, globals and config overlay, on the caller's budget. File-only, never a source string ([ADR 0006](adr/0006-isolated-script-execution.md)) |
| Type system | **Static, mandatory, explicit.** Every binding declares a type and its type never changes; conversions are explicit and checked; unions and intersections as in PHP; `mixed` is the one unchecked position. `uint` added alongside signed `int`; `float` is always `f64`. Arrays keep PHP's ordered hash but every key is a `string` and the element type may be declared and nested (`array<array<uint>>`) ([ADR 0007](adr/0007-explicit-type-system.md)) |
| Enums | **A closed, named integer type, C#-style — PHP's enum design is disregarded entirely.** `enum Status { Active, Banned }` declares cases as compile-time constants of an underlying `int` (default) or `uint`, auto-incrementing unless given a literal; no methods, no interfaces, no `::cases()`/`::from()`/`::tryFrom()`, no `string` backing, no runtime storage at all. The enum's name is a type usable anywhere ADR 0007 requires one — property, constant, parameter, local ([ADR 0010](adr/0010-enums-are-a-value-type.md)) |
| Scoping and state | **`static` is a class-member modifier only.** Static methods, static properties and late static binding (`static::`, `new static()`, `: static`) kept as PHP has them; function-scope `static` and `static fn` rejected with a diagnostic. No `global`. State that outlives a call lives in a class static, a constant, or an object property, and nowhere else ([ADR 0008](adr/0008-static-and-global.md)) |
| No superglobals | **No variable is ever populated by the host.** `$GLOBALS` and `$_REQUEST` are dropped with no replacement; every other PHP superglobal (`$_SERVER`, `$_GET`/`$_POST`/`$_COOKIE`/`$_FILES`, `$_SESSION`, `$_ENV`), the CLI SAPI's `$argv`/`$argc`, and MWL's own `$_ARGS` become `static` methods on reserved `Core` classes (`Core\Server`, `Core\Request`, `Core\Session`, `Core\Env`, `Core\Cli`, `Core\Script`), host-populated per isolate. Inside a spawned isolate, `Core\Request`/`Core\Server`/`Core\Session` throw rather than returning the parent's data or a fresh-and-empty result ([ADR 0012](adr/0012-no-superglobals.md)) |
| Object comparison | **Ordering two objects requires the global `Comparable` interface; PHP's ambient property-walk fallback is rejected outright.** `<`, `>`, `<=`, `>=` and `<=>` between two objects lower to a call to `compareTo(self $other): int`; a class that does not implement `Comparable` makes those operators a compile-time diagnostic, and two different classes are never directly orderable even when both implement it. `==`/`===`/`!=`/`!==` are untouched ([ADR 0013](adr/0013-comparable-interface.md)) |
| Property access | **A property's own hook runs first, then a declared `PropertyObserver` runs second — always both, never a fallback.** Per-property `get`/`set` hooks stay PHP 8.4's. A class implementing the global `PropertyObserver` interface additionally gets `onPropertyGet`/`onPropertySet` called, purely as an observer, after every property access, hooked or not; it cannot override the value. Accessing an undeclared property is always a hard error — a compile diagnostic for a literal name, a checked throw for a computed one — so PHP's `__get`/`__set` fallback for missing properties has nothing left to catch. `__call`/`__callStatic` are not recognized by name at all ([ADR 0014](adr/0014-property-observer.md)) |
| OOP-only: no free functions, no global constants | **Every callable is a method, every constant a class constant — no exception for built-ins.** `function` and `const` are rejected outside a class body. Built-ins live under `Core`, a reserved namespace organised into domain classes (`Core\Str`, `Core\Arr`, `Core\Math`, …) rather than one god class; `strlen($s)` becomes `Core\Str::len($s)`, `PHP_EOL` becomes `Core\Env::EOL`, reached via ordinary `use`/fully-qualified resolution with nothing auto-imported. Anonymous functions and arrow functions are unaffected — they are values, not named declarations ([ADR 0011](adr/0011-functions-and-constants-are-class-members.md)) |
| PHP compatibility | **Pragmatic superset of the syntax, not of the type discipline.** PHP 8.5 syntax accepted; `strict_types` implicit; no `eval`, `$$var`, `goto`, `global`, `extract()`, `settype()`, function-scope `static` or `static fn`. `eval` and `exec('php …')` have a replacement rather than only a rejection: `spawn script` ([ADR 0006](adr/0006-isolated-script-execution.md)). PHP has no syntax for the type of a local, so existing PHP does **not** run unconverted — `mwl convert` writes the annotations ([ADR 0007](adr/0007-explicit-type-system.md) lists the nine deliberate divergences; [ADR 0010](adr/0010-enums-are-a-value-type.md) adds a tenth for `enum`, since MWL's is not PHP's class-like construct at all; [ADR 0011](adr/0011-functions-and-constants-are-class-members.md) adds an eleventh — PHP's global function and global constant declarations do not exist in MWL at all; [ADR 0012](adr/0012-no-superglobals.md) adds a twelfth — no PHP superglobal exists as a variable, `$GLOBALS` and `$_REQUEST` have no replacement at all; [ADR 0013](adr/0013-comparable-interface.md) adds a thirteenth — ordering two objects with `<`/`>` no longer falls back to PHP's implicit property walk; [ADR 0014](adr/0014-property-observer.md) adds a fourteenth — `__get`/`__set` no longer fire for an undefined property, since one no longer exists to fire for, and `__call`/`__callStatic` are gone with no replacement) |
| Templating | `<?mwl … ?>` inline-HTML mode, `<?= ?>` short echo, `.mwl` extension. Explicit escaping (not auto) |
| Request state | **Strict shared-nothing.** Only compiled code survives a request. No connection pooling in v1 (seam reserved). A request is the root isolate of a tree; `spawn script` adds children to it |
| Regex | Pure Rust two-tier: `regex` (linear-time) → `fancy-regex` (lookaround/backrefs) fallback |
| Security | Server-level `mwl.ini`, root-owned, php.ini-style, deny-by-default capabilities + hard per-request limits. Per-directive changeability: capabilities tighten-only, limits freely settable per request up to a `System` ceiling ([ADR 0005](adr/0005-config-changeability.md)) |
| Serving | Built-in HTTP/1.1 + h2c server. FastCGI deferred to optional transport. HTTP/3 out of scope |
| Databases | MySQL/MariaDB, PostgreSQL, SQLite, MS SQL Server |
| Tooling | LSP + formatter, test runner, debugger + profiler, package manager |
| Testing | Hand-written suite is normative; `.phpt → .mwlt` transpiler imports PHP's corpus |
| Migration | `mwl convert` — real PHP→MWL transpiler |
| Extensions | Three tiers: built-in, sandboxed **WebAssembly components** (`.mwlx`), statically linked native. No `dlopen` |
| Platforms | Windows x86_64, Linux x86_64, macOS (x86_64 + aarch64) |
| Licence | MIT |

### Rationale for the two calls left open

**Built-in HTTP server over FastCGI.** FastCGI's worst vulnerability class — the
`SCRIPT_FILENAME`/`PATH_INFO`/`cgi.fix_pathinfo` RCE family — exists *because* the decision of which file
to execute is split between web server and runtime. A native server keeps that decision in one place.
Throughput over loopback/UDS differs by single-digit microseconds per request, irrelevant beside script
execution. A bespoke FCGI record parser would be attack surface we own; `hyper` is memory-safe and among
the most-fuzzed HTTP stacks in existence. Isolation guarantees live in the host, not the protocol, so
transports sit behind a trait and FCGI can be added later for shared hosting/IIS.

**HTTP/1.1 + h2c, no HTTP/3.** nginx `proxy_pass` speaks HTTP/1.1 upstream only → h1 keep-alive is
mandatory. Caddy/Traefik/HAProxy/Envoy support cleartext h2 upstream, and `hyper` provides h1+h2 in one
crate → h2c is nearly free. No production proxy speaks HTTP/3 to an origin; the edge terminates QUIC and
talks h1/h2 upstream → h3 is pure cost.

### Consequences to accept

- **`Hello World` is ~3–5 weeks out, not day one.** With no interpreter tier, the first program requires
  the whole front end *plus* a working native backend. Mitigation: the backend ships as a *baseline* tier
  where every operation lowers to a call into a Rust runtime helper — mechanically close to an interpreter
  loop, so it is fast to get correct, and typed inlining layers on afterwards without redesign.
- **Strict shared-nothing means reconnecting to the database every request.** That is a real per-request
  cost frameworks will feel. The host will expose a `PersistentRegistry` seam (unused in v1) so pooling can
  be added later without architectural change.
- **A JIT means a native-codegen component in the trusted core.** User programs stay fully memory-safe
  (all codegen is type-checked and bounds-checked); the codegen itself, the coroutine stack switcher and
  the arena are the audited unsafe surface. See "Unsafe policy".
- **Resident memory is higher than a footprint-tuned runtime's, and scales with in-flight concurrency
  rather than request rate.** 64 KiB of reserved stack per task, 16-byte values, copy-on-write clones and a
  per-request arena held to its peak are all deliberate purchases of safety, speed or simplicity. Sizing a
  deployment therefore means sizing for concurrency — tasks × stack, plus concurrent requests × their
  `memory` cap — and deployment docs must say so rather than quote a typical RSS.
- **Existing PHP does not run unconverted.** PHP has no syntax for the type of a local, a `foreach`
  binding or a destructuring target ([ADR 0007](adr/0007-explicit-type-system.md)), and every one of its
  global functions and global constants needs a new home on a class before it type-checks at all
  ([ADR 0011](adr/0011-functions-and-constants-are-class-members.md)). So the migration story is
  `mwl convert` writing annotations and rewriting call sites into the source, not dropping `.php` files
  into a document root. That moves type inference out of the compiler and into a one-time source rewrite a
  human reviews — better placed, but it makes M11 mandatory rather than a convenience, and it lowers the
  imported `.phpt` pass rate structurally rather
  than through bugs.
- **One new language construct that PHP has no equivalent of.** `spawn script` is a surface a developer
  has to learn and the spec has to define next to `include`, which they will confuse it with. Accepted:
  the requirement it answers — run another file, isolated, without a second process — has no other honest
  answer, and it reuses the request boundary and the worker value rules rather than adding either
  ([ADR 0006](adr/0006-isolated-script-execution.md)).
- **This is an 18–24 month effort at full-time pace for one experienced engineer**, front-loaded: M0–M4
  (a usable CLI language) is roughly 3–4 months; the stdlib and DB drivers are the long tail.

---

## Validated premises

The M0 spikes are now permanent guard tests in [`benches/abi-probe`](../benches/abi-probe/), because
several decisions here rest on how Cranelift, `corosensei` and Wasmtime behave rather than on MWL's own
code, and a dependency bump can invalidate them silently. **The tests own the numbers; this document does
not restate them.**

| Premise | Guarded by | Argued in |
|---|---|---|
| Native unwinding through JIT frames is unavailable on every platform — the premise the calling convention exists for | `tests/unwind_unavailable.rs` | [0002](adr/0002-error-propagation.md) |
| A throw propagates, and a runtime panic is *contained*, across native frames | `tests/invariants.rs` | [0002](adr/0002-error-propagation.md) |
| A checked-return frame stays cheap, and throwing costs no more than returning | `a_checked_return_frame_stays_cheap`, `throwing_costs_about_the_same_as_returning` | [0002](adr/0002-error-propagation.md) |
| A coroutine can suspend from beneath live JIT frames, cheaply | `a_helper_can_suspend_with_jit_frames_live_above_it`, `a_coroutine_round_trip_stays_cheap` | [adr/README.md](adr/README.md), *Stackful coroutines* |
| A wasm guest cannot read past the host heap or outlive its deadline, and the boundary is affordable | `tests/wasm_sandbox.rs`, `a_host_to_guest_call_stays_cheap`, `per_request_instantiation_stays_affordable` | [0003](adr/0003-extension-system.md) |
| An OS process still costs orders of magnitude more than a task — the whole cost case for in-process isolation | `an_os_process_costs_orders_of_magnitude_more_than_a_task` | [0006](adr/0006-isolated-script-execution.md) |

Those spikes forced one design change, and it is normative for everything below: **exceptions propagate by
checked return, not by unwinding**, and every runtime helper is `extern "C"` wrapping `catch_unwind`. The
signature, the measured cost and the reasoning are in [ADR 0002](adr/0002-error-propagation.md).

## Extension system

Three tiers, each the right answer for a different class of code rather than a compromise. The full
reasoning — the WIT interface, the handle-table value model, the measured boundary costs, the isolation and
loading rules, and the decisive rejection of `dlopen` — is in [ADR 0003](adr/0003-extension-system.md).

- **Tier 0 — built-in (`mwl-stdlib`).** Compiled into the binary, native, direct heap access, no boundary.
  Home of the fine-grained primitives whose total cost is comparable to a call.
- **Tier 1 — WebAssembly component extensions (`.mwlx`).** The default for third parties: one binary that
  runs on every platform, sandboxed by construction, authorable in any language `wit-bindgen` targets.
- **Tier 2 — statically linked native.** A Rust crate compiled into a custom `mwl` binary, for first-party
  subsystems needing raw sockets, TLS or direct heap access — `mwl-db`, `mwl-regex`, crypto. It requires
  building from source, which is the right friction for code that runs unsandboxed.

One sequencing constraint, which is why this section is in the plan at all: the `mwl:ext@1.0.0` WIT world
must be authored **during** the stdlib milestone (M8), from the same value-access design as the built-ins,
so that the Tier 0 internal interface and the Tier 1 guest interface are one design rather than two that
drift. Deferring it to M9 would mean retrofitting.

## Architecture

```
                    .mwl / .php source
                            │
    ┌───────────────────────▼───────────────────────┐
    │ mwl-syntax    lexer (dual mode, inline HTML)  │
    │               parser → AST + spans            │
    ├───────────────────────────────────────────────┤
    │ mwl-hir       name resolution, namespaces,    │
    │               class graph, symbol table       │
    ├───────────────────────────────────────────────┤
    │ mwl-types     declared types, unions, flow    │
    │               narrowing, no inference engine  │
    ├───────────────────────────────────────────────┤
    │ mwl-ir        CFG/SSA, safepoints, refcount   │
    │               ops, optimisation passes        │
    ├───────────────────────────────────────────────┤
    │ mwl-codegen   Cranelift → native code (W^X),  │
    │               status checks, helper calls     │
    └───────────────────────┬───────────────────────┘
                            │  Arc<CompiledUnit>  (immutable, shared)
    ┌───────────────────────▼───────────────────────┐
    │ mwl-host   unit cache (single-flight compile) │
    │            isolate = arena + globals + limits │
    │            (a request is one isolate's root)  │
    └───────┬───────────────────────────────┬───────┘
            │                               │
    ┌───────▼───────┐               ┌───────▼───────┐
    │ mwl-http      │               │ mwl-cli       │
    │ h1 + h2c      │               │ run / test    │
    └───────────────┘               └───────────────┘
            (mwl-fcgi later, same Transport trait)
```

### Thread-per-core, shared-nothing runtime

The decisive structural choice. One single-threaded Tokio runtime **pinned per CPU core**; connections are
load-balanced across cores; a request never migrates between cores.

- Value refcounts are **non-atomic** — no atomics on the hot path, because a heap is only ever touched by
  one thread.
- True multicore parallelism comes from N independent runtimes.
- `spawn worker` for CPU-bound work dispatches to another core; the value crossing the boundary is
  deep-copied (moved when refcount == 1). This is exactly the isolated-worker semantics already chosen, so
  the model is internally consistent rather than a compromise.
- Compiled units are immutable, so they are shared across all cores via `Arc` with zero copying.

### Value representation

16-byte tagged value: `{ tag: u8, _pad: [u8;7], bits: u64 }`. NaN-boxing is rejected because PHP semantics
require full-range `i64`. Tags: `null | bool | int(i64) | uint(u64) | float(f64) | string | array | object |
closure | resource`. `uint` is a tag, not a wider slot, so it costs nothing here; the type system that
demands it is [ADR 0007](adr/0007-explicit-type-system.md), which also owns the array element-type stamp
carried on the array header.

Memory: refcounting + copy-on-write arrays/strings (PHP semantics). Reference cycles are bounded by the
request lifetime — the whole request heap is dropped wholesale at request end, which makes cycle leaks
structurally impossible to accumulate in the server. Long-running CLI scripts additionally get an optional
mark-sweep cycle collector running at safepoints.

Both choices here — 16 bytes per value instead of 8, and peak-not-average retention inside a request — cost
memory to buy correct PHP semantics and a collector that never runs on the request path. That is the
priority ordering in [ADR 0004](adr/0004-memory-for-simplicity.md), not an oversight to optimise away later.

### Safepoints — build these into codegen from the very first commit

Codegen emits a cheap poll (load a per-task flag, branch) at every loop back-edge and function entry. This
single mechanism delivers:

1. CPU-time limit enforcement and wall-clock timeouts,
2. cancellation when a client disconnects,
3. cycle-collector and profiler stop-the-world points,
4. debugger breakpoints,
5. deoptimisation/OSR points for the optimising tier.

Retrofitting safepoints later would mean rewriting the backend. They are not optional.

### Compiled-unit cache with single-flight compilation

```rust
enum CompileState {
    Compiling(broadcast::Receiver<Result<Arc<CompiledUnit>, Arc<Diagnostics>>>),
    Ready(Arc<CompiledUnit>),
    Failed(Arc<Diagnostics>),
}
// DashMap<UnitKey { path, content_hash }, CompileState>
```

The first requester inserts `Compiling` and compiles on a **dedicated compile pool** (never on a
request-serving core, so compilation cannot stall request handling). Concurrent requesters await the same
broadcast — N simultaneous first-hits compile exactly once, and none of them block a core. Staleness:
`stat` (mtime+size) → BLAKE3 content hash → atomic swap. Governed by
`opcache.validate = never|mtime|hash`. Native pages are mapped `RX`, never `RWX` (W^X discipline).

### Per-request isolation

Each request gets: its own heap arena with a hard byte cap; fresh backing state for the `Core\Request`/
`Core\Server`/`Core\Session` accessors ([ADR 0012](adr/0012-no-superglobals.md), replacing PHP's
superglobals); a copy-on-write overlay of the config; its own coroutine tree. At request end the arena is
released
wholesale. `catch_unwind` at the request boundary means a runtime panic kills one request, never the
process, which is why `panic = "unwind"` is load-bearing in every profile. Every capability check
consults the *request's* config snapshot, so a script cannot affect its neighbours.

### In-process isolated script execution

Provisional surface — the spec pins it down in M5:

```php
$job    = spawn script 'jobs/report.mwl' with(args: ['month' => 7], limits: ['memory' => '256M']);
$result = await $job;                    // ScriptResult { ok, value, output, error, usage }
```

with the callee an ordinary script that reads `Core\Script::args()` and answers with a top-level `return`
([ADR 0012](adr/0012-no-superglobals.md)).

The semantics are decided and stated in full in [ADR 0006](adr/0006-isolated-script-execution.md): what is
shared (only immutable compiled code), how values cross (the same deep-copy-or-move rules and the same
implementation as cross-core worker dispatch), how budgets are accounted (at the root of the request tree,
never per isolate), the `script.spawn` capability and its path resolution, and failure arriving as a value
rather than as an unwind. The three invariants no optimisation may trade away are listed in
[CLAUDE.md](../CLAUDE.md).

The structural consequence for this plan: `mwl-host` gains **one** `Isolate` type, and an inbound HTTP
request *is* the root isolate of its tree. The server path (M7) and the `spawn script` path (M5) therefore
share one arena setup, one teardown, one place limits are enforced — and one state-bleed test suite. That
is why isolates land in M5, before the server that depends on them.

### `mwl.ini` — server-level, root-owned

php.ini-style directive registry, root-owned, with per-app capability blocks living in the *root* config so
an application can never grant itself rights. `mwl.ini` states **defaults, not ceilings**: a directive is a
limit that cannot be exceeded only when it cannot be changed at runtime at all. Each directive carries one
of three changeability classes — `System`, `Runtime`, `RuntimeTighten`.

[ADR 0005](adr/0005-config-changeability.md) holds the only copy of the directive layout: the classes and
why each is argued per directive, the `[core]`, `[limits]`, `[limits.hard]`, `[capabilities]` and `[app]`
sections, and the `ini_set`/`ini_get`/`ini_restore` overlay rules. Do not restate it here.

Two cross-cutting consequences the milestones below depend on:

- A widened limit lives on the request-local copy-on-write overlay, so it dies with the request that set it
  and is never visible to another. A set refused by a ceiling or a class returns `false` and leaves the
  value unchanged — it is **not** clamped.
- Every `[limits]` value is accounted against the **root of a request tree**, not per isolate, which is
  what keeps the process worst case independent of how many isolates a script creates
  ([ADR 0006](adr/0006-isolated-script-execution.md)).

---

## Repository layout

The layout, and which crates exist today versus which are deferred:
[README](../README.md#repository-layout). Crates for later milestones are created when their milestone
starts rather than sitting empty.

### Unsafe policy

`unsafe_code = "forbid"` workspace-wide. Crates that genuinely need it opt down to `deny` and allow
individual blocks with a stated reason: `mwl-runtime` and `mwl-codegen` (planned — the coroutine stack
switcher, the request arena, JIT page mapping), plus `benches/abi-probe`, which must call JIT-compiled code
to measure it and is `publish = false`, so it does not widen the runtime's unsafe surface. Those modules
carry `deny(unsafe_op_in_unsafe_fn)`, a safety-invariant doc comment per block, dedicated Miri/ASAN
coverage, and require an ADR to grow. Prefer `corosensei` (audited, handles Windows SEH and aarch64) over a
hand-rolled switcher. The policy is enforced in [Cargo.toml](../Cargo.toml).

---

## Milestones

Each milestone ends with something runnable and its own tests. Do not start the next until the current
one's verification passes.

### M0 — Project setup (~3 days) — **done**
Workspace scaffold, the CI matrix across the three platforms, `clippy -D warnings`, `rustfmt`,
`cargo-deny`, `cargo-fuzz`, the licence, and the first ADRs recording the decision table above. The
architecture spikes were promoted into `benches/abi-probe` as permanent guard tests rather than left in a
scratchpad.

**Verified:** `cargo test` / `cargo clippy` / `cargo deny check` green on all three platforms in CI.

### M1 — Front end (~3 weeks)
Lexer with dual mode (`<?mwl`, `<?php`, `<?=`), inline HTML, heredoc/nowdoc, string interpolation, all
PHP 8.5 tokens. Recursive-descent parser covering the pragmatic-superset grammar: classes, interfaces,
traits, enums (cases and an optional backing type only — no methods, no `implements`, see
[ADR 0010](adr/0010-enums-are-a-value-type.md)), methods (a `function` declaration is only ever a class
member, static or instance — see [ADR 0011](adr/0011-functions-and-constants-are-class-members.md)),
attributes, `match`, closures and arrow functions, generators, named arguments, spread, nullsafe,
`readonly`, promoted constructor parameters, first-class callable syntax, property hooks (their pipeline
relative to the new `PropertyObserver` interface is [ADR 0014](adr/0014-property-observer.md)), asymmetric
visibility. Rejects `eval`/`$$var`/`goto`/`global`/`extract`/`settype`/function-scope `static`/`static
fn`/enum methods/`enum … implements`/`enum … : string`/a `function` or `const` declared outside a class
body/a `namespace` or class named `Core` (or nested under it)/any superglobal spelling (`$GLOBALS`,
`$_SERVER`, `$_GET`, `$_POST`, `$_COOKIE`, `$_FILES`, `$_REQUEST`, `$_SESSION`, `$_ENV`, `$argv`, `$argc`)
with a diagnostic naming the replacement ([ADR 0012](adr/0012-no-superglobals.md)). Error recovery good
enough for the LSP.

Plus the type grammar of [ADR 0007](adr/0007-explicit-type-system.md), which is a parser problem before it
is a checker one: nested `array<T>`, DNF unions and intersections, `uint`, the conversion operator, and the
declaration slots PHP has no syntax for — typed locals, `foreach` bindings and destructuring targets.

**Verify:** `mwl ast file.mwl` dumps the AST; `insta` snapshot tests; `cargo fuzz` on the lexer and parser
finds no panic in a 1h run; parse the full local PHP 8.5 install's `.php` files without crashing (they will
not *check* — see M2 — but they must parse). A snapshot pins the one grammar wrinkle in ADR 0007: `as` in a
`foreach` header belongs to `foreach`, so a conversion of the subject needs parentheses.

### M2 — HIR, types, IR (~4 weeks)
Name resolution, namespaces and `use`, class hierarchy with trait flattening, statically resolved
`require`/`include` with a dynamic fallback. Every callable and constant resolves as a class member — there
is no bare-name fallback in the resolver at all — and a declaration reusing the reserved `Core` namespace is
a diagnostic at that site ([ADR 0011](adr/0011-functions-and-constants-are-class-members.md)). The same
resolver refuses a property access naming anything not declared on the class or an ancestor/trait, for
every literal-identifier access — there is no `__get`/`__set` fallback for a missing property, since one
cannot exist under this rule ([ADR 0014](adr/0014-property-observer.md)). The type checker of
[ADR 0007](adr/0007-explicit-type-system.md): every binding's declared type recorded and enforced,
definite-assignment checking, flow-sensitive narrowing of unions, array element types checked at every
write and at every nesting depth, the arithmetic result-type table including the refusal of `int + uint`,
and interned type descriptors. There is **no inference engine and no `Unknown` type** — that is the
simplification the mandatory declarations buy. Also from that table: `<`/`>`/`<=`/`>=`/`<=>` between two
objects refused unless both sides are provably the same class implementing `Comparable`
([ADR 0013](adr/0013-comparable-interface.md)) — there is no property-walk fallback to fall into. Lowering
to a CFG/SSA IR carrying explicit safepoints, refcount operations and runtime-helper calls.

**Verify:** `mwl check` on a curated corpus where every diagnostic named in ADR 0007 is its own file — an
undeclared local, a re-declared local, a read before definite assignment, `int + uint`, `int $n = 7 / 2;`,
a `mixed` assigned into a typed binding, an element-type violation at depth 1, 2 and 3, a missing narrowing
and a present one. IR snapshot tests. No program in the corpus produces an `Unknown` type, because the IR
no longer has one. `< > <= >= <=>` on two objects diagnosed exactly per [ADR 0013](adr/0013-comparable-interface.md):
refused when the class does not implement `Comparable`, refused across two different classes even when
both do.

### M3 — Baseline Cranelift backend → **Hello World** (~3 weeks)
The checked-return calling convention from [ADR 0002](adr/0002-error-propagation.md), which is normative
for the signature, the `catch_unwind` helper wrapper and the status check emitted after every call. There
is no platform unwind-table registration to do — that is the point of that ADR. Plus the runtime helper
table, `echo`, string concat, arithmetic, comparison, control flow, function calls, safepoint polls and
W^X page management.

Because [ADR 0007](adr/0007-explicit-type-system.md) makes operand types known by construction, the
baseline tier emits a native instruction wherever the static type is a single scalar and falls back to the
generic helper only for `mixed`, unions and dynamic calls. That is a chunk of what M12 was for, arriving
with the first backend.

**Verify:** `mwl run hello.mwl` prints `Hello World` from natively compiled code on all three platforms. A
throw crosses several JIT frames and is caught; a helper panic terminates the script with a `FATAL` status
and leaves the process able to run the next one. An MWL-level backtrace names the right functions, resolved
from MWL's own frame chain rather than from the platform unwinder. `mwl run --dump-asm` shows generated
code. A typed arithmetic loop lowers to native instructions rather than helper calls, committed as a figure
in `benches/` with a guard, so ADR 0007's claim that mandatory types pay for themselves on the request path
is tested rather than asserted.

### M4 — Language completeness — a usable CLI language (~10 weeks)
Full ordered-hash arrays with COW — string-only keys, insertion order, declared element types enforced at
every write ([ADR 0007](adr/0007-explicit-type-system.md)) — `uint` arithmetic with its overflow throws and
its logical `>>`, the conversion operator over every row of that ADR's conversion table, exceptions
propagating correctly by checked return across JIT frames ([ADR 0002](adr/0002-error-propagation.md)),
closures that bind
`$this` only where the body uses it ([ADR 0008](adr/0008-static-and-global.md)),
inheritance/interfaces/traits, the built-in global `Comparable` interface lowering `< <= > >= <=>` between
two objects to `compareTo`, with no property-walk fallback ([ADR 0013](adr/0013-comparable-interface.md)),
enums as a closed named integer type with cases inlined as compile-time
constants ([ADR 0010](adr/0010-enums-are-a-value-type.md)), generators (nearly free given stackful
coroutines), `foreach`
and iterators, references (`&$x`), instance members and static members including late static binding
(`static::`, `new static()`, `: static`), property hooks feeding the built-in global `PropertyObserver`
interface with a hard error on any undeclared property and no `__call`/`__callStatic` at all
([ADR 0014](adr/0014-property-observer.md)), the first `Core` domain classes'
`static` methods for string/array/math operations
([ADR 0011](adr/0011-functions-and-constants-are-class-members.md)), `var_dump`/`print_r`/`json_encode`.

Also in this milestone: `mwl test` and the `.mwlt` format — deliberately defined as a **superset of
`.phpt` sections** (`--TEST--`, `--FILE--`, `--EXPECT--`, `--EXPECTF--`, `--SKIPIF--`, `--INI--`,
`--ARGS--`, `--ENV--`, `--CLEAN--`) so the M11 importer is mechanical rather than a rewrite.

**Verify:** hand-written conformance suite ≥ 1000 `.mwlt` cases green, including `uint` at `0`, `i64::MAX`,
`i64::MAX + 1` and `2^64 − 1`; every conversion in ADR 0007 both succeeding and throwing; overflow throwing
rather than promoting to `float`; key order preserved across insert, delete, re-insert and every sort
function; `array_keys()` typed `array<string>`; `json_encode` output identical to PHP's for both lists and
maps. `new static()` through two levels of inheritance returns the called class, and a closure written in a
method without naming `$this` is unbound — `bindTo()` on it rebinds nothing, which is ADR 0008's single
divergence and gets its own case. A class implementing `PropertyObserver` runs its `onPropertyGet`/
`onPropertySet` after each property's own hook or storage, for hooked and un-hooked properties alike, and
cannot override the value; a class that does not implement it shows no measurable overhead over plain field
access ([ADR 0014](adr/0014-property-observer.md)). A non-trivial CLI program (an argument-parsing
file-processing tool) runs correctly; no leaks under Valgrind/ASAN.

### M5 — Concurrency and script isolates (~5 weeks)
Per-core runtimes, coroutine scheduler, `spawn` / `await` / `all` / `race` / `timeout`, `Channel` with
backpressure, `parallel_map`, cross-core worker dispatch with deep-copy-or-move, structured concurrency
(a task tree dies with its parent — no orphans), async-native file I/O, sockets, timers and HTTP client.

**Also in this milestone: `spawn script`** ([ADR 0006](adr/0006-isolated-script-execution.md)) — the
`Isolate` type in `mwl-host` with its own arena, `Core` accessor backing state and config overlay; the
request tree and its shared budget; `Core\Script::args()` ([ADR 0012](adr/0012-no-superglobals.md)) and the
top-level `return` contract; the `ScriptResult` shape; the value-crossing
rules shared with worker dispatch (graph copy, refusal of closures, references and resources, refusal of an
unresolvable class); `output: capture|inherit`; `on: worker`; cancellation of a child at its next safepoint.
It belongs here rather than later because it is a task with a heap boundary, which is exactly what this
milestone builds — and doing it now means the HTTP server in M7 is written against the same `Isolate`
instead of growing a second isolation path that has to be unified afterwards. Enforcement of its *limits*
and of the `script.spawn` capability lands with the rest of the config work in M6; until then it runs under
compiled-in defaults.

**Verify:** stress tests with 100k concurrent tasks; a deliberate deadlock test proves cancellation
works; `parallel_map` shows near-linear speedup across cores on a CPU-bound benchmark; ThreadSanitizer
clean. For isolates: a child cannot read or write a parent variable, global, static, or output buffer, and
a `Core\Request`/`Core\Server`/`Core\Session` call inside it throws rather than seeing the parent's request
([ADR 0012](adr/0012-no-superglobals.md)); a closure, reference or resource is refused at the boundary; a
cyclic argument crosses without
hanging; a child's uncaught throw, its limit breach and a contained panic inside it all leave the parent
running with `ok = false`; a cancelled parent leaves no orphan and no leaked arena; spawn-to-result for a
trivial child on a warm cache is single-digit microseconds, committed to `benches/isolation.rs` next to the
process baseline it replaces, with a guard test alongside
`an_os_process_costs_orders_of_magnitude_more_than_a_task`.

### M6 — Config, limits, capabilities, disk cache (~3 weeks)
Directive registry with changeability classes, boot config parsing, per-request overlay, `ini_set`
semantics, capability enforcement at every syscall-touching stdlib entry point, safepoint-driven limit
enforcement, content-addressed artifact cache with integrity verification and a refusal to use a
world-writable cache directory. Isolates get their governance here: the `script.spawn` capability with
canonicalise-then-prefix path resolution, `max_script_depth`, per-tree accounting of every `[limits]` value,
spawn-site sub-caps, and derivation of a child's overlay from its parent's effective config.

**Verify:** adversarial suite — a script attempting to widen a capability or set a `System` directive
fails; `ini_set('memory', '512M')` above the `[limits]` default succeeds and takes effect, above the
`[limits.hard]` ceiling returns `false` with the previous value intact, and is invisible to the next request
on the same core; memory/CPU caps terminate runaway scripts with a catchable error; warm-cache CLI startup
under 10 ms; a tampered cache artifact is rejected. For isolates: `spawn script` without `script.spawn`
fails; a path outside the granted roots fails, including one reaching it through `..` or a symlink; a child
cannot widen a capability its parent narrowed; N concurrent isolates cannot together exceed the tree's
memory, CPU or output budget; a recursive spawn is stopped by `max_script_depth` and reported as that rather
than as an out-of-memory.

### M7 — Built-in HTTP server (~4 weeks)
`mwl serve`: hyper h1 + h2c, per-core accept and dispatch, request → the root isolate of a request tree
(the same `Isolate` M5 built, not a second isolation path), the `Core\Request`/`Core\Server` accessor
classes populated from it (`Core\Request::query()`/`::post()`/`::cookie()`/`::file()`,
`Core\Server::meta()`/`::header()` — replacing `$_GET`/`$_POST`/`$_SERVER`/`$_COOKIE`/`$_FILES`, see
[ADR 0012](adr/0012-no-superglobals.md)), multipart and urlencoded body parsing with limits,
streaming responses, static-file serving, graceful shutdown and zero-downtime reload, structured request
logging, optional TLS via `rustls`.

**Verify:** the core requirement demonstrated under load — 10k concurrent cold requests for the same file
compile it **exactly once** (assert via a compile counter) with no stalled requests; a state-bleed test
suite proves nothing leaks between requests, and the same suite runs across an isolate boundary, which the
shared `Isolate` makes a parameterisation rather than a second suite; a request whose isolates are still
running when the client disconnects leaves none of them behind; path traversal, header injection and
request-smuggling suites pass; `wrk`/`oha` throughput compared against PHP 8.5 + FPM + opcache and recorded
in `benches/`.

### M8 — Stdlib and databases (~16 weeks)
Two-tier regex with the `preg_*` layer; JSON; hashing and crypto (RustCrypto: sha2, blake3, argon2,
bcrypt, aes-gcm); date/time with PHP-compatible formatting; filesystem and stream abstractions; process
execution behind the capability gate; sessions; a PDO-like DB API with pure-Rust MySQL/MariaDB, PostgreSQL
and MS SQL Server drivers plus SQLite (documenting `rusqlite`'s C dependency as an explicit, audited
exception to the pure-Rust rule).

**Also in this milestone: author the `mwl:ext@1.0.0` WIT world.** It must be designed from the same
value-access model as the `Core` domain classes' static methods, so the Tier 0 internal interface and the Tier 1 guest
interface are one design rather than two that drift. Writing it later would mean retrofitting. The same
applies to the signatures themselves: the parametric array signatures
([ADR 0007](adr/0007-explicit-type-system.md) — `array_map(callable, array<T>): array<U>` and friends) are
written once for the built-ins and reused by the WIT world, where `uint` now maps to `u64` with no
conversion. Type variables stay available only to declarations the compiler owns; user-defined generics are
not part of this milestone.

**Verify:** per-subsystem conformance suites; DB drivers tested against real servers in CI containers,
including TLS, prepared statements, transactions and large result streaming.

### M9 — Extension system (~6 weeks)
`mwl-ext`: `.mwlx` loading (wasm component + `mwl.manifest` custom section), manifest parsing and
registration into the compiler symbol table so extension calls are statically type-checked, the WIT host
implementation, per-call handle tables for value access, lazy per-request instantiation on the pooling
allocator, epoch-interruption wiring to the per-request CPU cap, `StoreLimits` wiring to the memory cap,
the capability bridge (no ambient authority; optional WASI world with preopens derived from `mwl.ini`
grants), hash pinning and signature verification, and compiled-module caching in the existing
content-addressed artifact cache.

Tooling: `mwl ext new --lang rust|c|zig|go`, `mwl ext build` (one portable `.mwlx`), `mwl ext inspect`
(manifest and requested capabilities), `mwl ext test`, `mwl ext verify`.

**Verify:** a real extension end to end — an image codec or compression library — built from Rust *and*
from a second language to prove the toolchain claim, running unmodified on all three platforms from one
binary. Adversarial suite: an extension attempting filesystem or network access it was not granted fails;
a runaway extension is trapped by the request's CPU cap rather than hanging a core; a deliberately
memory-hungry extension hits the cap; an extension that stores state in a global cannot observe it on the
next request. Benchmark in-guest compute throughput against the equivalent native Tier 2 implementation and
**commit the numbers** — this is the one figure in ADR 0003 that is currently asserted rather than
measured.

### M10 — Developer tooling (~12 weeks)
`mwl fmt` (canonical, idempotent); `mwl lsp` over `tower-lsp` reusing the front end with incremental
reparse (completion, go-to-definition, hover types, diagnostics, rename); `mwl dap` using safepoints for
breakpoints plus deopt-to-debug in codegen; a sampling profiler emitting flamegraphs; `mwl pkg` with
lockfile, semver resolution and a registry.

**Verify:** VS Code and PhpStorm both drive the LSP; `mwl fmt` is idempotent across the whole corpus;
breakpoints hit in JIT-compiled code with correct variable values; profiler output attributes time to the
right MWL functions.

### M11 — PHP transpiler (~10 weeks)
`mwl convert`: PHP source → AST → rewrite passes → idiomatic `.mwl` output. **This milestone is now on the
critical path for adoption rather than a convenience**, because PHP has no syntax for a local's type and
[ADR 0007](adr/0007-explicit-type-system.md) requires one: the converter carries the type-inference engine
MWL's compiler deliberately does not have, and writes the annotations into the output for a human to review.
Where inference cannot decide, it emits `mixed` with a `TODO` naming the binding rather than guessing — an
honest `mixed` runs, and a wrong annotation would not. That inference pass is the reason for the two extra
weeks over the original estimate.

Mechanical rewrites where possible (`global` → parameter passing, function-scope `static` → a
`private static` property on the owning class or a parameter where there is no class, `static fn` → the
keyword dropped ([ADR 0008](adr/0008-static-and-global.md)), `extract()` → explicit assignment,
`settype()` → a second binding or an `as` conversion, lossy `(int)` casts flagged where the source relied
on PHP's silent `0`, simple `$$var` → match on a map, `exec('php script.php …')` job dispatch →
`spawn script`, which is a real rewrite rather than a `TODO` because the isolation the original bought is
what the construct provides, a backed enum's case declarations and `->value` reads → an MWL `enum` and
an `as` conversion ([ADR 0010](adr/0010-enums-are-a-value-type.md)), and a call or reference to a PHP
built-in global function or constant (`strlen`, `array_map`, `PHP_EOL`, …) → the matching `Core`
class-and-member, `Core\Str::len`, `Core\Arr::map`, `Core\Env::EOL`, via a maintained PHP-name → `Core`
table that grows with the stdlib ([ADR 0011](adr/0011-functions-and-constants-are-class-members.md))); a
rewrite that needs a human look because it changes the shape of the surrounding code (a source file's own
top-level `function`/`const` declarations, with no built-in counterpart, are grouped into one generated
class named after the file — the same "needs a class to hang it on" shape the function-static rewrite
already has, per [ADR 0011](adr/0011-functions-and-constants-are-class-members.md)); annotated `TODO`
diagnostics where neither applies (`eval` of constructed source, dynamic includes, unsupported `preg`
constructs, a `bindTo()` whose target closure never names `$this` — the one divergence ADR 0008 introduces,
and visible here rather than at run time — an enum that implements an interface or declares a method, which
has no mechanical destination under [ADR 0010](adr/0010-enums-are-a-value-type.md), a class declaring
`__get`/`__set` that needs a human call on whether the original logic was observation (→ `PropertyObserver`)
or computation (→ a per-property hook), a class declaring `__call`/`__callStatic` with no mechanical
destination at all ([ADR 0014](adr/0014-property-observer.md)), and C extensions).
`--check` mode emits a migration report without writing files. A `.phpt → .mwlt` converter
reuses the same pipeline to import PHP's test corpus as native MWL tests. A PHP project depending on a C
extension is reported as needing either a Tier 1 `.mwlx` replacement or a Tier 2 native one — the converter
cannot synthesise either, and says so rather than emitting code that fails at runtime.

**Verify:** convert a real open-source PHP project end to end and run its test suite under MWL, reporting
*annotations written* against *`TODO`s emitted*; imported `.phpt` cases run in `mwl test` with a tracked
pass rate and failures triaged as bug vs intentional divergence — the nine type-discipline divergences in
[ADR 0007](adr/0007-explicit-type-system.md) §7 are counted separately, or the structural gap reads as
regression.

### M12 — Optimising JIT tier (ongoing)
Profiling counters, inline caches for property and method access (monomorphic → polymorphic →
megamorphic), unboxed int/uint/float fast paths, inlining, refcount elision, escape analysis, deopt and OSR
at existing safepoints. Narrower than originally scoped: the declared types of
[ADR 0007](adr/0007-explicit-type-system.md) mean the baseline tier is already typed, so speculation only
has to cover `mixed`, unions and dynamic calls.

**Verify:** macro benchmarks show a multiple over the baseline tier and over PHP 8.5 with JIT; no
correctness regressions in the full conformance suite when the optimising tier is forced on.

### M13 — Optional FastCGI transport
Only if a deployment target requires it (shared hosting, IIS, an existing nginx estate). Implements the
same `Transport` trait as `mwl-http`, with a fuzzed record parser and `SCRIPT_FILENAME` handling that
resolves within a configured root — closing the historical vulnerability class by construction.

---

## Overall verification strategy

- **Unit** — `cargo test` per crate; `insta` snapshots for AST/IR/codegen.
- **Conformance** — hand-written `.mwlt` suite as the normative definition of MWL; imported `.phpt`
  corpus tracked as a compatibility percentage.
- **Property/fuzz** — `proptest` for the array and string implementations; `cargo-fuzz` on lexer, parser,
  HTTP parser, multipart, regex and JSON, run continuously in CI.
- **Sanitisers** — Miri on the safe subset, ASAN/TSAN on the unsafe core and the scheduler.
- **Security** — the adversarial suites from M6/M7 (capability escape, resource exhaustion, cross-request
  *and* cross-isolate state bleed, execution-root escape, traversal, smuggling) plus `cargo deny` advisories
  on every build.
- **Performance** — `criterion` microbenchmarks and application-level macro benchmarks, always compared
  against the locally installed PHP 8.5.8, with results committed so regressions are visible in diffs.
