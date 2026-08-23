# MWL — Modern Web Lang: Implementation Plan

<!-- This block has a fixed field set: Status, Done, On disk, Toolchain, ADR slices landed, Open now,
     Blocking. Overwrite a field in place; never add a paragraph or a new field name. That is what
     keeps it bounded as milestones accumulate. Aim for ~400 bytes a field — guidance for you, not a
     check: nothing verifies it, and no session should ever be spent trimming to a number. History
     lives in `git log`, per-crate gaps in each crate's module doc — see AGENTS.md's "Writing docs
     here" section. -->

> **Status:** 2026-08-23. **M3 is done**, and the loop goal's Stages 1-2 plus `examples/report.mwl` are
> green on Windows and Linux alike. Current: **M4**, run with **M4S** in one loop — `Core\Arr`'s contract rests on M4's copy-on-write
> array, and building that array without its only real consumer produces one that must be rebuilt. M4's
> language surface is now driven by what Stage 3's `Core` work needs.
>
> **Done:** M0 (setup); M1 (front end — lexer with dual mode, inline HTML, heredoc/nowdoc and
> interpolation, the full parser, and the M1-scoped grammar of ADRs
> 0024/0031/0033/0034/0035/0036/0037/0049/0050; `crates/mwl-syntax/tests/corpus_parse.rs` parses the
> local `php-src` checkout and a 5-minute
> WSL `cargo fuzz run lex`/`parse` both find zero panics).
>
> **On disk:** the workspace, CI on three platforms, lint/deny/fmt/notice policy, `mwl-diagnostics`,
> `mwl-syntax`, `mwl-hir`, `mwl-types` (+ `layout`, `core_lib`, `error_lib`, `iter_lib`, `generics`,
> `conformance`, `defaults`), `mwl-ir`, `mwl-runtime` (+ `object`, `array`, `throwable`, `closure`),
> `mwl-stdlib` (`Arr` × 9, `Str` × 13, `Order`), `mwl-codegen`, `mwl-cli`, `fuzz/`, `tools/`,
> `benches/abi-probe`.
>
> **Toolchain:** Rust 1.97.1 stable (pinned), Cranelift 0.135.0, wasmtime 48, MSVC 14.44 + Windows SDK
> 10.0.26100 for linking, PHP 8.5.9 as the differential oracle, `cargo-fuzz` 0.13.2 and `valgrind` under a
> WSL nightly toolchain (AGENTS.md says why).
>
> **ADR slices landed:** checker-side rules for ADRs 0007, 0010, 0013, 0014, 0015, 0021, 0022, 0024,
> 0027, 0028, 0029/0030/0032, 0033, 0036, 0037, 0038, 0062, and 0043's syntax + default/private-method
> slice; end-to-end for 0007 §§ 2 and 4's `%` row, 0010, 0013, 0014 § 1, 0023 § 1, 0035 § 4, 0031 §§ 1-2,
> 0065, and **0053 in full**. Each ADR's own *Verification* section says what its slice covers, not this
> field.
>
> **Open now:** Stage 3 — the rest of `Core` §§ 1–12 as registry rows. An options bag, a union parameter,
> a callback-bound result type, a `Core`-owned enum and an absent option all work end to end.
> `examples/core.mwl` has one unblock left: `Str::length`'s ADR 0009 granularity. Then
> `crates/mwl-test`/`mwl test`. Off path: `for`/`switch`, ADR 0043's `by`-delegation.
>
> **Blocking:** nothing external. **Stages 1, 2 and `report.mwl` are green on both legs** — byte for
> byte on Windows and under WSL against a Linux build, `valgrind --leak-check=full` clean on all twelve
> fixtures. That leg earned its keep: `report.mwl` found a real per-iteration leak in `mwl-ir`, now
> fixed. `examples/core.mwl` now runs its **line 11's sort** and stops at `Str::length`.

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

Every row that names an ADR states only the headline — open that ADR for the mechanism, the exact
spellings rejected, and the reasoning. Do not restate that detail here when adding a row.

| Area | Decision |
|---|---|
| Implementation language | Rust (stable, pinned via `rust-toolchain.toml`) |
| Resource priorities | Security → semantics → latency → simplicity → memory footprint, in that order, within an enforced per-request cap ([ADR 0004](adr/0004-memory-for-simplicity.md)) |
| Execution | Cranelift JIT from day one, no interpreter tier; baseline codegen first, optimising tier later |
| Code cache | Content-addressed on-disk cache (BLAKE3) + in-process `Arc` sharing; hot-reloads on an edit via a per-path pointer swap, no watcher, no restart ([ADR 0017](adr/0017-hot-reload-without-restart.md)); the on-disk file format, its mmap-verify-then-execute read path and its eviction policy are [ADR 0042](adr/0042-on-disk-artifact-cache-format.md) |
| Parallelism | Hybrid: `async`/`await` for I/O inside a task (same heap, cooperative) + isolated workers on other cores for CPU work |
| Suspension | Stackful coroutines — no async colouring; any function may yield |
| Isolated execution | `spawn script 'file.mwl'` runs another file in-process as a child isolate, file-only, never a source string ([ADR 0006](adr/0006-isolated-script-execution.md)) |
| Type system | Static, mandatory, explicit; every binding's declared type never changes; `uint` alongside signed `int` ([ADR 0007](adr/0007-explicit-type-system.md)) |
| Enums | A closed, named integer type, C#-style; PHP's class-like enum design (`::cases()`, methods, `string` backing) is disregarded entirely ([ADR 0010](adr/0010-enums-are-a-value-type.md)) |
| Scoping and state | `static` is a class-member modifier only; no function-scope `static`, no `static fn`, no `global` ([ADR 0008](adr/0008-static-and-global.md)) |
| No superglobals | No variable is ever populated by the host; every PHP superglobal becomes a `Core` accessor class, and `$GLOBALS`/`$_REQUEST` have no replacement ([ADR 0012](adr/0012-no-superglobals.md)) |
| Object comparison | Ordering two objects requires the global `Comparable` interface; PHP's ambient property-walk fallback is rejected outright ([ADR 0013](adr/0013-comparable-interface.md)) |
| Property access | A property's own hook runs first, then a declared `PropertyObserver` second, always both, never a fallback for a missing property ([ADR 0014](adr/0014-property-observer.md)) |
| OOP-only: no free functions, no global constants | Every callable is a method, every constant a class constant; built-ins live under `Core` domain classes ([ADR 0011](adr/0011-functions-and-constants-are-class-members.md)) |
| Name aliasing | No `class_alias` or import `as`; a compile-time-only `type` alias for a type expression is the one exception ([ADR 0015](adr/0015-no-name-aliasing.md)) |
| Code reuse | No `trait`; shared behavior is a `public`/`private` interface method body, shared state is explicit `implements Interface by $field;` delegation, and any resulting name collision is always a compile error requiring an explicit override — there is no `insteadof` ([ADR 0043](adr/0043-interface-default-methods-and-delegation-replace-traits.md)) |
| PHP compatibility | Pragmatic superset of the syntax, not of the type discipline: PHP 8.5 syntax accepted, `strict_types` implicit, no `eval`/`$$var`/`goto`/`extract()`/`settype()`/pipe operator (`\|>`, deliberately unparsed — see `mwl-syntax`'s module docs). Existing PHP does not run unconverted — see *Consequences to accept* below, and each ADR above for its own divergence from PHP |
| Templating | `<?mwl … ?>` inline-HTML mode, `<?= ?>` short echo, `.mwl` extension. Explicit escaping (not auto) |
| Request state | Strict shared-nothing: only compiled code survives a request; no connection pooling in v1 (seam reserved). A request is the root isolate of a tree; `spawn script` adds children to it |
| Regex | Pure Rust two-tier: `regex` (linear-time) → `fancy-regex` (lookaround/backrefs) fallback |
| Security | Server-level `mwl.toml`, root-owned, TOML ([ADR 0064](adr/0064-configuration-file-format.md)), deny-by-default capabilities + hard per-request limits ([ADR 0005](adr/0005-config-changeability.md)) |
| Serving | Built-in HTTP/1.1 + h2c server. FastCGI deferred to optional transport. HTTP/3 out of scope |
| Databases | MySQL/MariaDB, PostgreSQL, SQLite, MS SQL Server |
| Tooling | LSP + formatter, test runner, debugger + profiler, package manager |
| Testing | Hand-written suite is normative; `.phpt → .mwlt` transpiler imports PHP's corpus |
| Migration | `mwl convert` — real PHP→MWL transpiler |
| Extensions | Three tiers: built-in, sandboxed **WebAssembly components** (`.mwlx`), statically linked native. No `dlopen` ([ADR 0003](adr/0003-extension-system.md)) |
| Platforms | Windows x86_64, Linux x86_64, macOS (x86_64 + aarch64) |
| Licence | MIT |

One decision — `string` is guaranteed-valid UTF-8, `bytes` is the separate binary type — is drafted but not
yet in this table: it is **Proposed**, not Accepted, pending a cost measurement
([ADR 0009](adr/0009-string-and-bytes.md)).

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
- **Existing PHP does not run unconverted.** PHP has no syntax for the type of a `foreach` binding or a
  destructuring target ([ADR 0007](adr/0007-explicit-type-system.md)), and every one of its global
  functions and global constants needs a new home on a class before it type-checks at all
  ([ADR 0011](adr/0011-functions-and-constants-are-class-members.md)). A plain local has a type-eliding
  spelling now — [ADR 0037](adr/0037-var-local-type-inference.md)'s `var` — so `mwl convert` can emit that
  directly instead of inferring and writing an annotation, but it still has to write annotations and
  rewrite call sites for everything else, not just drop `.php` files into a document root. M11 stays
  mandatory rather than a convenience, and the imported `.phpt` pass rate is still structurally lower than
  a compatibility-first design's, not through bugs.
- **One new language construct that PHP has no equivalent of.** `spawn script` is a surface a developer
  has to learn and the spec has to define next to `require`, which they will confuse it with. Accepted:
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
mark-sweep cycle collector running at safepoints; whichever milestone implements its run routine also gives
it a `gc`-kind trace event, at no cost to the safepoint poll itself
([ADR 0041](adr/0041-timeline-export-and-gc-spawn-trace-events.md)).

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

Because the key above is `{path, content_hash}`, not `{path}`, a `Ready` entry is write-once — two versions
of a file are two entries, never one overwritten. A small separate index, `path → current content_hash`,
sits in front of it and is the one thing a hot-reload actually swaps; [ADR 0017](adr/0017-hot-reload-without-restart.md)
holds the only copy of that mechanism, why revalidation needs no filesystem watcher, and how a file edited
under a live server reaches the next request with no restart while a request already running keeps the
version it started with.

### Per-request isolation

Each request gets: its own heap arena with a hard byte cap; fresh backing state for the `Core\Request`/
`Core\Server`/`Core\Session` accessors ([ADR 0012](adr/0012-no-superglobals.md), replacing PHP's
superglobals); a copy-on-write overlay of the config; its own coroutine tree. At request end the arena is
released
wholesale. `catch_unwind` at the request boundary means a runtime panic kills one request, never the
process, which is why `panic = "unwind"` is load-bearing in every profile. Every capability check
consults the *request's* config snapshot, so a script cannot affect its neighbours.

### In-process isolated script execution

The provisional surface, the semantics, and what is/is not shared are decided and stated in full in
[ADR 0006](adr/0006-isolated-script-execution.md) — including the `spawn script … with(…)` example, how
values cross (the graph-copy operation [ADR 0023](adr/0023-clone-serialize-and-cross-boundary-copy.md) now
formally defines, shared with `serialize()`/`unserialize()`), how budgets are accounted (at the root of the
request tree, never per isolate), the `script.spawn` capability and its path resolution, and failure
arriving as a value rather than as an unwind. The spec pins the exact grammar down in M5. The three
invariants no optimisation may trade away are listed in [AGENTS.md](../AGENTS.md).

The structural consequence for this plan: `mwl-host` gains **one** `Isolate` type, and an inbound HTTP
request *is* the root isolate of its tree. The server path (M7) and the `spawn script` path (M5) therefore
share one arena setup, one teardown, one place limits are enforced — and one state-bleed test suite. That
is why isolates land in M5, before the server that depends on them.

### `mwl.toml` — server-level, root-owned

A directive registry in a root-owned TOML file, with per-app capability blocks living in the *root* config so
an application can never grant itself rights. `mwl.toml` states **defaults, not ceilings**: a directive is a
limit that cannot be exceeded only when it cannot be changed at runtime at all. Each directive carries one
of three changeability classes — `System`, `Runtime`, `RuntimeTighten`.

[ADR 0005](adr/0005-config-changeability.md) holds the only copy of the directive layout: the classes and
why each is argued per directive, the `[core]`, `[limits]`, `[limits.hard]`, `[capabilities]` and `[app]`
tables, and the `Core\Config::set`/`::get`/`::restore` overlay rules.
[ADR 0064](adr/0064-configuration-file-format.md) holds the only copy of why the file is TOML rather than
INI, and of the `Core\Config` signatures. Do not restate either here.

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
switcher, the request arena, JIT page mapping), `mwl-stdlib`, whose every `Core` member is an
[ADR 0002](adr/0002-error-propagation.md) helper entry point and therefore an `extern "C"` function
decoding raw pointers, plus `benches/abi-probe`, which must call JIT-compiled code
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
enums (cases and an optional backing type only — no methods, no `implements`, see
[ADR 0010](adr/0010-enums-are-a-value-type.md)), methods (a `function` declaration is only ever a class
member, static or instance — see [ADR 0011](adr/0011-functions-and-constants-are-class-members.md)),
attributes, `match`, `fn` closures — with or without a block body, and with an optional self-name for
recursion, but no anonymous `function(...) {...}` literal and no `use` clause at all
([ADR 0031](adr/0031-callable-is-the-only-closure-type.md)) — generators, named arguments, spread, nullsafe,
`readonly`, promoted constructor parameters, first-class callable syntax, property hooks (their pipeline
relative to the new `PropertyObserver` interface is [ADR 0014](adr/0014-property-observer.md)), asymmetric
visibility. Rejects, with a diagnostic naming the replacement, every construct an earlier ADR closes:
`eval`/`$$var`/`goto`/`global`/`extract`/`settype`/function-scope `static`/`static fn`
([ADR 0008](adr/0008-static-and-global.md)), anonymous `function(...) {...}`/`function(...) use (...) {...}`
and any `use` capture clause ([ADR 0031](adr/0031-callable-is-the-only-closure-type.md)), enum
methods/`implements`/`string` backing
([ADR 0010](adr/0010-enums-are-a-value-type.md)), a `function` or `const` outside a class body and a
`namespace` or class named `Core` ([ADR 0011](adr/0011-functions-and-constants-are-class-members.md)),
every superglobal spelling ([ADR 0012](adr/0012-no-superglobals.md)), and `use … as …`
([ADR 0015](adr/0015-no-name-aliasing.md)). Error recovery good enough for the LSP.

Plus the type grammar of [ADR 0007](adr/0007-explicit-type-system.md), which is a parser problem before it
is a checker one: nested `array<T>`, DNF unions and intersections, `uint`, the conversion operator, and the
declaration slots PHP has no syntax for — typed locals, `foreach` bindings and destructuring targets.
**Added after this milestone's grammar work first landed:** the `tainted` qualifier on `string`/`bytes`
([ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md)) is new reserved-keyword grammar too — `tainted`
prefixing either scalar atom, everywhere a type may appear (parameter, return, property, local,
`foreach` binding). It belongs here for the same reason `uint` does: *enforcing* it is M2's job, but
*parsing* it is this milestone's, so it needs to land before M1's own verification below counts as
complete — a real, if small, addition discovered after the parser was first reported done. Also
new here: `type Name = TypeExpr;` ([ADR 0015](adr/0015-no-name-aliasing.md)), a file/namespace-scope
declaration using the same grammar, parsed but not yet resolved — that is M2's job.

**Added after that, a second time:** the `secret` qualifier on `string`/`bytes`
([ADR 0033](adr/0033-secret-qualifier-for-confidential-values.md)) is a second, independent reserved-keyword
qualifier alongside `tainted` — `secret string`, `secret bytes`, and, composed with `tainted`, `secret
tainted string`/`secret tainted bytes` (only in that order; the reverse is a diagnostic). Same reasoning as
`tainted`'s own addition above: parsing it is this milestone's job, enforcing it is M2's.

**Added after that, a third time:** [ADR 0036](adr/0036-anonymous-object-shapes.md) §§ 2-3 adds the
anonymous object-literal expression (`{a: 1, b: 2}`, no shorthand, no computed key) and an inline
structural shape type (`{name: T, ...}`) usable anywhere a type may appear. Two grammar collisions this
creates — `fn() => {...}` already meaning a block body (ADR 0031), and a statement-initial `{` already
meaning a block statement — are resolved the same way JavaScript resolves the identical `() => {...}`
ambiguity: parenthesize to force the expression reading, diagnosed by name at both call sites when a
non-empty literal is attempted without the parentheses. Parsing and this disambiguation are this
milestone's job; `object`'s real subtyping and the shape's structural check are M2's.

**Added after that, a fourth time — implemented in a follow-up session:**
[ADR 0043](adr/0043-interface-default-methods-and-delegation-replace-traits.md) removes `trait`, class-body
`use TraitName, ...;`, and `insteadof` from the grammar entirely (each is now a parse-time
`E_TRAIT_NOT_SUPPORTED` diagnostic naming the replacement) and adds two small extensions in their place: an
interface method may carry a body (a `public` default or a `private` helper — this fell out of the existing
shared class-body grammar with no parser change needed), and one entry in a class's `implements` list may
carry an optional `by $field` delegation suffix (a new `ImplementsClause` AST node). This retroactively
narrows the "classes, interfaces, traits, enums" grammar line above — `mwl-syntax`'s originally-shipped
`TraitDecl`/`UseTraitMember`/`TraitAdaptation*`/`TraitMethodRef` AST nodes and their parser/casing support
are gone, not merely superseded. `mwl-hir`/`mwl-types`'s own now-stale trait-flattening code was removed in
the same session, just to keep the workspace building — the new default/private-method and `by`-delegation
*resolution* those crates still need is unrelated follow-up work, tracked in that ADR's own *Consequences*
and *Verification* rather than reopening this "done" milestone's checkbox.

**Verify:** `mwl ast file.mwl` dumps the AST; `insta` snapshot tests; `cargo fuzz` on the lexer and parser
finds no panic in a 5 minute run; parse the full local `php-src` folder for `.php` files without crashing (they will
not *check* — see M2 — but they must parse). A snapshot pins the one grammar wrinkle in ADR 0007: `as` in a
`foreach` header belongs to `foreach`, so a conversion of the subject needs parentheses.

### M2 — HIR, types, IR (~4 weeks)
Name resolution: namespace/`use` scoping, class hierarchy resolution — no trait flattening, since traits do
not exist; instead, default-method/private-method visibility and `by`-delegation type-matching
([ADR 0043](adr/0043-interface-default-methods-and-delegation-replace-traits.md); this replaces
`crates/mwl-hir`'s already-written `hierarchy.rs` trait-use/`insteadof` resolution, per that ADR's
*Consequences*) — statically resolved `require` with a dynamic fallback
([ADR 0021](adr/0021-single-file-inclusion-construct.md)), `autoload`-declared name-to-file resolution as a
fixpoint over that same require-graph worklist
([ADR 0061](adr/0061-compile-time-autoload-and-program-discovery.md)), every callable/constant resolved as a class
member with no bare-name fallback ([ADR 0011](adr/0011-functions-and-constants-are-class-members.md)),
`type`-alias substitution ([ADR 0015](adr/0015-no-name-aliasing.md)), and property-access resolution with
no `__get`/`__set` fallback ([ADR 0014](adr/0014-property-observer.md)).

Type checker ([ADR 0007](adr/0007-explicit-type-system.md)): declared types enforced, definite assignment
(locals, and per [ADR 0022](adr/0022-definite-property-initialization.md) every constructor-declared
property), flow-sensitive union narrowing, array element types checked at every depth, the arithmetic
result-type table. No inference engine and no `Unknown` type — that's the simplification the mandatory
declarations buy. Also enforced here: `Comparable`-gated object ordering
([ADR 0013](adr/0013-comparable-interface.md)); `tainted` poisoning/laundering
([ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md)) and, independently, `secret`
([ADR 0033](adr/0033-secret-qualifier-for-confidential-values.md)); `callable` accepted only from
first-class-callable syntax or an `fn` literal, no `__invoke`
([ADR 0027](adr/0027-callable-is-closures-only.md), [ADR 0031](adr/0031-callable-is-the-only-closure-type.md));
`Stringable`-gated string conversion and `unset()` refused on a declared property
([ADR 0028](adr/0028-closing-the-remaining-magic-methods.md)); identifier casing
([ADR 0029](adr/0029-identifier-casing-is-checked.md)/[0030](adr/0030-no-leading-underscores-constructor-spelling.md)/[0032](adr/0032-acronym-casing-rule-revoked.md),
lands in `mwl-syntax` directly since it needs no name resolution); `object` subtyping and shape-type
structural checking ([ADR 0036](adr/0036-anonymous-object-shapes.md)); `foreach` accepted over exactly an
`array<T>`, an `Iterable<T>` or an `Iterator<T>`, with `$obj[$k]` on a non-array refused
([ADR 0053](adr/0053-iteration-and-generators.md)); and `decimal`'s conversion and arithmetic rows,
including `decimal + float` refused on the same grounds as `int + uint`
([ADR 0054](adr/0054-decimal-scalar-type.md), whose literals are target-typed with no suffix, so `as T`
must place one); and the nullable target form `as ?T`, which needs no parser work and reuses § 6's `?T`
narrowing, refusing the conversions that cannot fail or do not exist
([ADR 0066](adr/0066-nullable-conversion-operator.md)).

Lowering to a CFG/SSA IR carrying explicit safepoints, refcount operations and runtime-helper calls, with a
stable per-statement/per-edge id reserved for
[ADR 0018](adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)'s probes — cheap now,
expensive to retrofit once M3 builds on the IR without it. The IR must also be able to represent a
**suspension point inside a loop body**, so that [ADR 0053](adr/0053-iteration-and-generators.md)'s
state-machine lowering of a generator can be added without reshaping it. The transform itself may land
later; foreclosing it here is the expensive mistake, exactly as with the probe ids.

**Verify:** `mwl check` on a curated corpus with one fixture per diagnostic named in each ADR listed
above's own *Verification* section, plus ADR 0007's own core entries (undeclared/redeclared local,
read-before-definite-assignment, `int + uint`, array element-type violations at depth, a missing vs.
present narrowing). Plus IR snapshot tests; no program in the corpus produces an `Unknown` type, since
the IR has none.

### M3 — Baseline Cranelift backend → **Hello World** (~3 weeks)
The checked-return calling convention from [ADR 0002](adr/0002-error-propagation.md), which is normative
for the signature, the `catch_unwind` helper wrapper and the status check emitted after every call. There
is no platform unwind-table registration to do — that is the point of that ADR. Plus the runtime helper
table, `echo`, string concat, arithmetic, comparison, control flow, function calls, safepoint polls, the
debug-flags probe checks at every statement boundary and call site, and W^X page management
([ADR 0018](adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md) — same cost class as the
safepoint poll, landing with it rather than after it).

Because [ADR 0007](adr/0007-explicit-type-system.md) makes operand types known by construction, the
baseline tier emits a native instruction for any single-scalar static type and falls back to the generic
helper only for `mixed`, unions and dynamic calls — a chunk of what M12 was for, arriving with the first
backend.

**Verify:** every bullet below, machine-checked by the unattended loop against the frozen `examples/*.mwl`
fixtures on Windows and Linux — `docs/agent/loop-goal.md` holds the one copy of that acceptance list and is
authoritative for it. `mwl run` prints from natively compiled code. A throw crosses several JIT frames and
is caught; a helper panic terminates the script with a `FATAL` status and leaves the process able to run the
next one. An MWL-level backtrace names the right functions, resolved from MWL's own frame chain rather than
from the platform unwinder, and is readable from MWL as a rendered string — the `backtrace` member's array
form is a deliberate carry-over to M4, where arrays land. `mwl run --dump-asm` shows generated code. A
typed arithmetic loop lowers to native instructions rather than helper calls, committed as a figure in
`benches/` with a guard, so ADR 0007's claim that mandatory types pay for themselves on the request path is
tested rather than asserted.

### M4 — Language completeness — a usable CLI language (~10 weeks)
Full ordered-hash arrays with COW — including `Throwable`'s `backtrace` member, carried over from M3, which
lands its rendered string form only — `uint` arithmetic, and the conversion operator over every row of
[ADR 0007](adr/0007-explicit-type-system.md)'s conversion table; exceptions propagating correctly by
checked return across JIT frames ([ADR 0002](adr/0002-error-propagation.md)), closures that bind `$this`
only where the body uses it ([ADR 0008](adr/0008-static-and-global.md)),
inheritance/interfaces, including default/private interface method bodies and `by`-delegation
([ADR 0043](adr/0043-interface-default-methods-and-delegation-replace-traits.md)), object ordering through
`Comparable` ([ADR 0013](adr/0013-comparable-interface.md)), enums
([ADR 0010](adr/0010-enums-are-a-value-type.md)), `decimal` arithmetic
([ADR 0054](adr/0054-decimal-scalar-type.md)), generators and `foreach` over the two iteration interfaces
([ADR 0053](adr/0053-iteration-and-generators.md)),
references (`&$x`), instance members and static members including late static binding
(`static::`, `new static()`, `: static`), property hooks and `PropertyObserver`
([ADR 0014](adr/0014-property-observer.md)), `clone`
([ADR 0023](adr/0023-clone-serialize-and-cross-boundary-copy.md)), `var_dump`/`print_r`/`json_encode`
— with a `secret`-qualified property's value redacted
([ADR 0033](adr/0033-secret-qualifier-for-confidential-values.md)) — `Stringable` and the rest of
[ADR 0028](adr/0028-closing-the-remaining-magic-methods.md), and `#[...]` attribute syntax on every
declaration it attaches to, including the explicit `<T>` call-site type argument
`Core\Attributes::get<T>`/`::all<T>` need even though their retrieval body doesn't land until M8
([ADR 0046](adr/0046-attributes-shape-literal-metadata.md)).

Also in this milestone: `mwl test` and the `.mwlt` format — deliberately a **superset of `.phpt` sections**
(`--TEST--`, `--FILE--`, `--EXPECT--`, `--EXPECTF--`, `--SKIPIF--`, `--INI--`, `--ARGS--`, `--ENV--`,
`--CLEAN--`), so the M11 importer is mechanical rather than a rewrite.

**Verify:** hand-written conformance suite ≥ 1000 `.mwlt` cases green, including `uint` at `0`, `i64::MAX`,
`i64::MAX + 1` and `2^64 − 1`; every conversion in ADR 0007 both succeeding and throwing; overflow throwing
rather than promoting to `float`; key order preserved across insert, delete, re-insert and every sort
function; `array_keys()` typed `array<string>`; `json_encode` output identical to PHP's for both lists and
maps. `new static()` through two levels of inheritance returns the called class, and a closure written in a
method without naming `$this` is unbound — `bindTo()` on it rebinds nothing, which is ADR 0008's single
divergence and gets its own case. Plus one fixture per rule in the *Verification* section of ADRs
[0014](adr/0014-property-observer.md), [0023](adr/0023-clone-serialize-and-cross-boundary-copy.md),
[0028](adr/0028-closing-the-remaining-magic-methods.md) and
[0046](adr/0046-attributes-shape-literal-metadata.md) — each of those sections is the one home for what its
own rule requires, including the "a class that does not implement `PropertyObserver` shows no measurable
overhead" measurement. A non-trivial program runs correctly and leaks nothing under Valgrind — the
*argument-parsing, file-processing* half of that program moves to M8 with `Core\Cli` (§ 15) and `Core\IO`
(§ 14), since neither argv nor a file handle is reachable before capabilities exist at M6.

### M4S — The `Core` API contract and its pure half (~5 weeks)
The library the language has been compiling calls *against* since M2 without any of it existing. Its shape
is [ADR 0063](adr/0063-core-api-conventions.md) and its member list is
[docs/spec/01-core-library.md](spec/01-core-library.md), which is authoritative for every signature; this
milestone implements **§§ 1–12** of that file — `Core\Str`, `Arr`, `Math`, `Time`, `Json`, `Regex`,
`Encoding`, `Bytes`, `Path`, the three collection types, the exception types, `Random`, `Uuid`, `Hash`,
`Uri`, `Validate`, `Csv`, `Out`. Every one is pure: no capability, no reactor, no driver, no open handle, so
none of it is blocked on M5–M7. § 13's compiler-facing surfaces are pure too but each waits on something
outside `Core`; that file's own *Milestones* section says which, and is the one home for it. It is placed here rather than at M8 so that everything after it — the LSP's
completion data, M5's concurrency tests, M9's extension conformance fixtures, M11's converter mapping
table — is written against a real standard library instead of against fixtures that will need rewriting.
Part II of the spec file (anything capability-bearing) stays at M8 and merely conforms to the same
contract. `crates/mwl-stdlib` starts here — the Tier 0 crate
[ADR 0003](adr/0003-extension-system.md) § *Tier 0* already names, and the workspace manifest already
declares; `Core\Regex` binds the engine [ADR 0056](adr/0056-regex-engine-policy.md)
picks, and `Core\Time`'s `format`/`parse`/`shift` plus `Core\Str::format` land as
[ADR 0057](adr/0057-intrinsic-literal-folding.md) intrinsics with the compile-time half wired into
`mwl-types`.

**Verify:** every member in the spec file has a conformance test, and a mechanical check over that file
enforces the rules that can be checked mechanically — [ADR 0063](adr/0063-core-api-conventions.md)'s
*Verification* section is the one home for that list. PHP 8.5 is the differential oracle wherever a member
claims PHP-compatible observable behaviour (`Core\Str`, `Core\Arr`, `Core\Math`, `Core\Regex`), and each
deliberate divergence is a named fixture rather than a failing comparison. A `tainted` value cannot reach a
sink and cannot be laundered except by the members the spec marks **launder**. `Core\Arr` mutates in place
when its argument's refcount is 1 — measured, since it is the whole cost argument for
[ADR 0063](adr/0063-core-api-conventions.md) R3 — and allocates a copy when it is not. The M4 CLI program
is rewritten against `Core` and gets shorter.

### M4B — Minimal `mwl-lsp` and the VS Code extension (~3 weeks)
Pulled ahead of M10 by [ADR 0040](adr/0040-vscode-deep-tooling-and-resilient-parsing.md) so real-world
testing in an editor starts the moment M4 makes MWL a usable CLI language, rather than after M5–M9.
`crates/mwl-syntax` gains a second, error-recovering parse entry point — a lossless tree, in the shape
rust-analyzer's `rowan` popularized, that keeps producing a usable structure around a syntax error instead
of aborting — used only by the pieces below; `mwl check`/`mwl run` keep the existing strict, all-or-nothing
parse unchanged. `crates/mwl-lsp` (`tower-lsp`) ships its first, minimal slice: diagnostics (via `mwl
check` run against the resilient tree), hover (declared types), go-to-definition, and keyword/member
completion — no workspace-wide symbol search or code actions yet, that's M10. `editors/vscode` ships
alongside it: `.mwl` registration, a TextMate grammar, `language-configuration.json`, `mwl lsp` process
spawning, a `LanguageStatusItem` for server health, `mwl run`/`mwl test` as VS Code Tasks, and an AST
explorer panel backed by the CLI's existing `mwl ast` command (no new language feature needed for that
one). No formatting support yet (`mwl fmt` doesn't exist until M10) and no PhpStorm work — PhpStorm stays
entirely at M10, per [ADR 0016](adr/0016-ide-integration.md).

**Verify:** typing an incomplete statement (unclosed brace, trailing `->`) does not stop
diagnostics/hover/completion from working on the well-formed code around it — the resilient-parse mode's
core claim. The VS Code extension activates on `.mwl`, shows TextMate colour immediately and semantic-token
colour once `mwl-lsp` responds, and diagnostics/hover/go-to-definition/completion round-trip through it
with no logic duplicated into the extension. The AST panel renders `mwl ast --json`'s tree for the active
file.

### M5 — Concurrency and script isolates (~5 weeks)
Per-core runtimes, coroutine scheduler, `spawn` / `await` / `all` / `race` / `timeout`, `Channel` with
backpressure, `parallel_map`, cross-core worker dispatch with deep-copy-or-move, structured concurrency
(a task tree dies with its parent — no orphans), async-native file I/O, sockets, timers and HTTP client.
`serialize()`/`unserialize()` share this milestone's deep-copy-or-move graph walk, externalized to MWL's own
closed byte format ([ADR 0023](adr/0023-clone-serialize-and-cross-boundary-copy.md)); `unserialize()` refuses
anything not in that format, with no `__serialize`/`__unserialize`/`__sleep`/`__wakeup` hook. That same graph
walk — both as `serialize()` and as the `spawn worker`/`spawn script` value-crossing operation below — refuses
a `secret`-qualified value outright unless it was first passed through `Core\Secret::reveal()`
([ADR 0033](adr/0033-secret-qualifier-for-confidential-values.md)).

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
compiled-in defaults. The three spawn-construct routines built here also each gain a `spawn`-kind trace
event with a parent/child overhead split, once ADR 0018's `TRACE`/`PROFILE` bits exist alongside them
([ADR 0041](adr/0041-timeline-export-and-gc-spawn-trace-events.md)).

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
`an_os_process_costs_orders_of_magnitude_more_than_a_task`. `serialize()`/`unserialize()` round-trip a cyclic
value using the same graph-copy fixtures as the isolate-boundary tests; bytes that are not MWL's own format,
or that name a class whose declared properties no longer match, are refused rather than partially accepted
([ADR 0023](adr/0023-clone-serialize-and-cross-boundary-copy.md)).

### M6 — Config, limits, capabilities, disk cache (~3 weeks)
Directive registry with changeability classes, boot config parsing (TOML via `serde`, with duplicate and
unknown keys refused — [ADR 0064](adr/0064-configuration-file-format.md)), per-request overlay,
`Core\Config::set` semantics, capability enforcement at every syscall-touching stdlib entry point, safepoint-driven limit
enforcement, content-addressed artifact cache with integrity verification and a refusal to use a
world-writable cache directory — the exact file layout, header format, mmap-verify-then-execute read path
and probabilistic eviction sweep are already decided in [ADR 0042](adr/0042-on-disk-artifact-cache-format.md);
this milestone builds exactly what that ADR specifies, not a fresh design. Isolates get their governance here: the `script.spawn` capability with
canonicalise-then-prefix path resolution, `max_script_depth`, per-tree accounting of every `[limits]` value,
spawn-site sub-caps, and derivation of a child's overlay from its parent's effective config. **Also here:**
the `fatal_reserve_memory`/`fatal_reserve_time` directives and `Core\Fatal::onLimit` registration
([ADR 0020](adr/0020-error-escalation-ladder.md)) — the reserved slice a resource-limit `FATAL`'s handler
runs with is carved out of the request's own budget at the same point these limits are set up.

**Also here: `mwl build --compile`**, a CLI-only "one portable executable" bundler — appends an entry file's
statically-resolved `require` graph to the host `mwl` binary as plain source, read back through this same
artifact cache with no new mechanism. Scope, the source-not-precompiled-artifacts trade, and why bundling a
web-serving deployment is explicitly out of scope are all in
[ADR 0048](adr/0048-portable-single-file-executables.md), the only copy of the reasoning.

**Verify:** adversarial suite — a script attempting to widen a capability or set a `System` directive
fails; `Core\Config::set('memory', '512M')` above the `[limits]` default succeeds and takes effect, above the
`[limits.hard]` ceiling returns `false` with the previous value intact, and is invisible to the next request
on the same core; memory/CPU caps terminate runaway scripts as a `FATAL`, reported to `Core\Fatal::onLimit`
if registered and never to an ordinary `catch` ([ADR 0020](adr/0020-error-escalation-ladder.md)); warm-cache
CLI startup under 10 ms; a tampered cache artifact is rejected. For isolates: `spawn script` without `script.spawn`
fails; a path outside the granted roots fails, including one reaching it through `..` or a symlink; a child
cannot widen a capability its parent narrowed; N concurrent isolates cannot together exceed the tree's
memory, CPU or output budget; a recursive spawn is stopped by `max_script_depth` and reported as that rather
than as an out-of-memory. For the bundler: a bundled executable runs identically to `mwl run` against the
same source, on all three platforms, per ADR 0048's own verification list.

### M7 — Built-in HTTP server (~4 weeks)
`mwl serve`: hyper h1 + h2c, per-core accept and dispatch, request → the root isolate of a request tree
(the same `Isolate` M5 built, not a second isolation path), the `Core\Request`/`Core\Server` accessor
classes populated from it (`Core\Request::query()`/`::post()`/`::cookie()`/`::file()`,
`Core\Server::meta()`/`::header()` — replacing `$_GET`/`$_POST`/`$_SERVER`/`$_COOKIE`/`$_FILES`, see
[ADR 0012](adr/0012-no-superglobals.md)), returning `tainted string`/`tainted bytes` per
[ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md), multipart and urlencoded body parsing with
limits, streaming responses, static-file serving, graceful shutdown and zero-downtime reload, structured
request logging, optional TLS via `rustls`. **Not yet named here:** raw/unparsed body access (a JSON
payload, a webhook body, an arbitrary content-type) — a gap [ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md)'s *Revisiting* flags for whoever designs `Core\Request`'s full surface at this milestone.

**Also in this milestone: hot-reload of the compiled-unit cache**, which is what makes "no restart to see an
edit" true of a running server rather than only of `mwl run`. [ADR 0017](adr/0017-hot-reload-without-restart.md)
holds the only copy of the mechanism — a per-path pointer over the content-addressed cache M5/M6 already
built, revalidated lazily and rate-capped, swapped without ever blocking a request-serving core. Every
request stays as isolated as a fresh subprocess regardless: compiled code is the only thing this milestone
ever lets one request share with another, and that sharing is exactly what M5's `Isolate` and M6's
`[limits]`/`[limits.hard]` already bound per request tree, not per file.

**Verify:** the core requirement demonstrated under load — 10k concurrent cold requests for the same file
compile it **exactly once** (assert via a compile counter) with no stalled requests; a state-bleed test
suite proves nothing leaks between requests, and the same suite runs across an isolate boundary, which the
shared `Isolate` makes a parameterisation rather than a second suite; a request whose isolates are still
running when the client disconnects leaves none of them behind; path traversal, header injection and
request-smuggling suites pass; `wrk`/`oha` throughput compared against PHP 8.5 + FPM + opcache and recorded
in `benches/`. For hot-reload specifically ([ADR 0017](adr/0017-hot-reload-without-restart.md)): editing a
file under concurrent load recompiles it exactly once no matter how many in-flight requests race to notice;
a request that resolved the old version runs it to completion while a newer version is already being served
to new requests; a revalidation that fails to compile fails only requests resolving it afterwards; a `stat`
storm against one hot, `mtime`-validated file is bounded by `revalidate_freq`, not by request rate.

### M8 — Stdlib and databases (~16 weeks)
**The roster this milestone builds is [ADR 0051](adr/0051-standard-library-tiers.md) § 3** — which class is
Core, which is a capability-gated native subsystem, which is an extension, and which of PHP's extensions
has no equivalent at all. That ADR is the one home for the list; this paragraph covers only what M8 must
decide beyond it.

Regex is two-tier, and the tiering is a rule rather than an implementation detail: a linear-time engine by
default, backtracking only for patterns it cannot express and only under a throwing step budget, with a
literal pattern's tier settled at compile time and the *pattern* argument refusing `tainted`
([ADR 0056](adr/0056-regex-engine-policy.md)). The compile-time half of that rides on
[ADR 0057](adr/0057-intrinsic-literal-folding.md)'s closed intrinsic list, which also lands here and
covers `Core\Uri`, the date-format strings and `Core\Str::format`'s placeholder checking.
`Core\Decimal`/`Core\BigInt`/`Core\BigDecimal` supply the method surface around the `decimal` scalar M2–M4
already built ([ADR 0054](adr/0054-decimal-scalar-type.md)) — including `divExact`, `divRound` and
`allocate`, since division is the one place a decimal result may be inexact. `Core\Cache`'s two tiers, the
copy-in/copy-out rule and the per-core memory cap are [ADR 0059](adr/0059-cross-request-state-is-explicit.md);
`Core\Http\Client`'s `tainted`-refusing URL parameter, the `Core\Http::allowUrl` launderer and the
`net.connect` address policy are [ADR 0058](adr/0058-outbound-request-policy.md); the closed
signed-cookie/CSRF/TOTP/JWT roster and its correct-by-construction constraints are
[ADR 0060](adr/0060-application-security-protocols.md).

Also: JSON; hashing and crypto (RustCrypto: sha2, blake3, argon2,
bcrypt, aes-gcm), AEAD-only per ADR 0051 § 3; date/time with PHP-compatible formatting; filesystem and
stream abstractions — with no scheme dispatch anywhere in them
([ADR 0052](adr/0052-closed-doors.md) § 2); process
execution behind the `process.exec` capability gate — `Core\Process::run()`/`::spawn()`, argv-only with no
shell-string form at all, a Windows batch/PowerShell-target refusal, and coroutine-suspending waits, per
[ADR 0044](adr/0044-core-process-argv-only-no-shell.md) (which supersedes ADR 0024 §4's original
placeholder bullet); sessions, which may not be backed by `Core\Cache`'s local tier; a PDO-like DB API with pure-Rust
MySQL/MariaDB, PostgreSQL and MS SQL Server drivers plus SQLite (documenting `rusqlite`'s C dependency as an
explicit, audited exception to the pure-Rust rule) — its query-text parameter requires the plain,
unqualified `string` while bound parameters stay tainted-friendly, and `Core\Html::escape`/`Markup` and the
`Core\Taint`/`Core\Db::quoteIdentifier` laundering functions land here too, all per
[ADR 0024](adr/0024-taint-tracking-for-injection-sinks.md). Also here: **`Core\Reflect` and `Core\Ast`**, built-in — not
extension-provided — structural reflection and a runtime door onto `mwl-syntax`'s own lexer/parser, per
[ADR 0019](adr/0019-reflection-and-ast-parsing-are-core-features.md); reflective access enforces the same
visibility/hook checks ordinary code does, and a parsed AST is typed, inert data with no path back into
execution. Also here: **`Core\Attributes`**, the narrow, statically-resolved `get<T>`/`all<T>` accessor onto
attribute literals attached in M4 — deliberately not part of `Core\Reflect`'s general-purpose walk, per
[ADR 0046](adr/0046-attributes-shape-literal-metadata.md). Also here: **`Core\Fatal` and `Core\Log`**, plus the operator-configured `.mwl` error-handler
script and the engine-native logging floor beneath it, per
[ADR 0020](adr/0020-error-escalation-ladder.md); `Core\Log`'s JSON-Lines writer is the same native
serialiser the engine floor calls directly, so the two never disagree on log shape. `mwl check` refuses a
`secret`-qualified operand at a `Core\Log::write()` call site's `fields` argument despite that parameter's
open `array<string, mixed>` type, and `Core\Secret::reveal()` plus the password-hashing helpers from this
milestone's crypto line item above are the two ways a value legitimately loses `secret` before reaching it
([ADR 0033](adr/0033-secret-qualifier-for-confidential-values.md)).

**Also in this milestone: author the `mwl:ext@1.0.0` WIT world.** It must be designed from the same
value-access model as the `Core` domain classes' static methods, so the Tier 0 internal interface and the Tier 1 guest
interface are one design rather than two that drift. Writing it later would mean retrofitting. **It must
also carry a qualifier axis** — a parameter that refuses `tainted`, and a return that is always `tainted` —
per [ADR 0055](adr/0055-extension-qualifier-declarations.md), which M9 then freezes; adding it afterwards
would be a breaking change to a published ABI, and without it any `.mwlx` launders untrusted data merely by
being called. The same
applies to the signatures themselves: the parametric array signatures
([ADR 0007](adr/0007-explicit-type-system.md) — `array_map(callable, array<T>): array<U>` and friends) are
written once for the built-ins and reused by the WIT world, where `uint` now maps to `u64` with no
conversion. Type variables stay available only to declarations the compiler owns; user-defined generics are
not part of this milestone.

**Verify:** ADRs 0051 and 0054–0060 each carry their own M8 verification list — that is the one home for
them, and this paragraph does not restate it. Two are worth naming here because they are CI infrastructure
rather than fixtures: a check enumerating the default binary's C dependencies, failing on any addition not
recorded against [ADR 0051](adr/0051-standard-library-tiers.md) § 4's two questions, and a check that no
class outside Tier 0 registers a name beginning `Core\`. Beyond that: per-subsystem conformance suites; DB drivers tested against real servers in CI containers,
including TLS, prepared statements, transactions and large result streaming. `Core\Reflect`/`Core\Ast`
verified per [ADR 0019](adr/0019-reflection-and-ast-parsing-are-core-features.md)'s own M8 verification
list — a reflective call to a `private` method from outside its class fails like the equivalent ordinary
call; `Core\Ast::parse()` fuzzed with the same corpus as M1's lexer/parser target. `Core\Attributes` verified per
[ADR 0046](adr/0046-attributes-shape-literal-metadata.md) — `get<T>` on a site with zero matching attributes
compiles to a constant `null`, one match compiles to that constant value with no runtime lookup, and more
than one match is a compile-time diagnostic naming `all<T>`; a literal property/parameter name that names no
real member is a compile-time diagnostic, and a non-literal one is a runtime empty result instead.
`Core\Fatal`/`Core\Log`
verified per [ADR 0020](adr/0020-error-escalation-ladder.md)'s own M7/M8 list — `onUncaughtThrow` receives
the real `Throwable`; the configured handler script runs charged to the engine's own reserve and still
fires when the reporting request is at its own memory ceiling; application code and the engine floor
produce schema-identical log records for the same error. `Core\Process` verified per
[ADR 0044](adr/0044-core-process-argv-only-no-shell.md)'s own M8 list — a tainted `$path`/`$argv` element is
a compile-time diagnostic, a Windows batch/PowerShell target is refused, the `process.exec` capability is
deny-by-default, and a concurrent-spawn scheduler guard sits alongside `benches/abi-probe`'s existing
coroutine-suspension tests.

### M9 — Extension system (~6 weeks)
`mwl-ext`: `.mwlx` loading (wasm component + `mwl.manifest` custom section), manifest parsing and
registration into the compiler symbol table so extension calls are statically type-checked — including the
`tainted`/`secret` qualifier axis, applied at an extension call site by the same code path as a `Core` one,
with a manifest attempting the laundering form ADR 0055 § 3 says does not exist failing validation at load
time — the WIT host
implementation, per-call handle tables for value access, lazy per-request instantiation on the pooling
allocator, epoch-interruption wiring to the per-request CPU cap, `StoreLimits` wiring to the memory cap,
the capability bridge (no ambient authority; optional WASI world with preopens derived from `mwl.toml`
grants), hash pinning and signature verification, and compiled-module caching in the existing
content-addressed artifact cache.

Tooling: `mwl ext new --lang rust|c|zig|go`, `mwl ext build` (one portable `.mwlx`), `mwl ext inspect`
(manifest and requested capabilities), `mwl ext test`, `mwl ext verify`.

**Verify:** the two first-party extensions [ADR 0051](adr/0051-standard-library-tiers.md) § 3 places at
Tier 1, end to end. The **image codec** is the one that makes the security claim legible — decoding an
attacker-supplied file in a sandbox with a memory cap and an epoch deadline — and it is built from Rust
*and* from a second language to prove the toolchain claim, running unmodified on all three platforms from
one binary. The **intl component** proves the two shapes that ADR's Ext placement depends on: CLDR data
carried in the component's own wasm data section, and a batch-shaped API where sorting 10,000 strings costs
one boundary crossing rather than one per comparison. Adversarial suite: an extension attempting filesystem or network access it was not granted fails;
a runaway extension is trapped by the request's CPU cap rather than hanging a core; a deliberately
memory-hungry extension hits the cap; an extension that stores state in a global cannot observe it on the
next request. Benchmark in-guest compute throughput against the equivalent native Tier 2 implementation and
**commit the numbers** — this is the one figure in ADR 0003 that is currently asserted rather than
measured.

### M10 — Developer tooling and IDE integration (~14 weeks; scope shifted by ADR 0040, net change undetermined)
`mwl fmt` (canonical, idempotent — the **only** formatting implementation; neither editor client below gets
its own; its PER-based, unconfigurable, no-reflow style and `--check`/`--diff` surface are
[ADR 0039](adr/0039-canonical-code-formatting.md)); `mwl-lsp` grows past M4B's minimal slice into full
workspace-wide symbol search, incremental reparse, rename, and code actions; `mwl dap` using safepoints for
breakpoints plus deopt-to-debug in codegen; a sampling profiler, emitting output in the open speedscope
format so it opens in existing viewers rather than a bespoke flamegraph renderer
([ADR 0040](adr/0040-vscode-deep-tooling-and-resilient-parsing.md)); `mwl pkg` with lockfile, semver
resolution and a registry. Also here: `Core\Debug`, the `[debug]` `mwl.toml` section and
`debug.trace`/`debug.profile` capabilities, and the Clover/lcov/Callgrind exporters wired to `mwl test
--coverage=…` and `mwl run --profile=…` — the developer-facing coverage/tracing/profiling feature whose
probe mechanism landed with M3
([ADR 0018](adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md), a deterministic per-call
profiler distinct from the sampling one above), plus a speedscope-evented export rendering that same
trace data — now tagged `call`/`gc`/`spawn` — as one scrollable timeline
([ADR 0041](adr/0041-timeline-export-and-gc-spawn-trace-events.md)).

**Also in this milestone: the rest of the two editor clients.** M4B already shipped `mwl-lsp`'s minimal
slice and `editors/vscode`'s baseline; what lands here per
[ADR 0016](adr/0016-ide-integration.md)/[ADR 0040](adr/0040-vscode-deep-tooling-and-resilient-parsing.md) is
the deep half:

- **`editors/vscode`** (extending the M4B package, not a second one) — format-on-save and format commands
  wired to `mwl fmt`; inspections and quick fixes for every ADR-named diagnostic that has an obvious fix
  (casing, legacy casts, missing property init, `include`→`require`, `tainted`/`secret` laundering);
  workspace-wide rename, extract-to-method/variable, alias-free organize-imports; signature help and
  cross-file completion; inlay hints; a native Test Explorer wired to `mwl test`/`.mwlt` with coverage via
  VS Code's own `FileCoverage` API (no custom gutter UI); a "View Profile" command opening the
  speedscope-format sampling output; and — reversing ADR 0016 § 4 for VS Code specifically — a
  `DebugAdapterDescriptorFactory` and `launch.json` schema wiring `mwl dap` into VS Code's existing debugger
  UI. Full rationale and per-feature dependencies: [ADR 0040](adr/0040-vscode-deep-tooling-and-resilient-parsing.md).
- **`editors/phpstorm`** — unchanged from [ADR 0016](adr/0016-ide-integration.md): a Kotlin/Gradle plugin
  that registers `.mwl` as its own file type (distinct from PhpStorm's bundled PHP support, which must not
  claim it), bridges to the **same** `mwl lsp`/`mwl fmt` binaries through JetBrains' LSP client support (or
  LSP4IJ, per ADR 0016 *Revisiting*), and ships an equivalent TextMate-or-equivalent baseline grammar.
  PSI-level refactoring, structural search, a native Formatter/Code Style page, a Test Explorer, and
  debugger UI wiring are all still out of scope for PhpStorm — [ADR 0016](adr/0016-ide-integration.md)
  names the native-plugin path as a later decision, not a silent gap, and [ADR 0040](adr/0040-vscode-deep-tooling-and-resilient-parsing.md)
  does not touch PhpStorm at all.

**Verify:** `mwl fmt` is idempotent across the whole corpus, and neither editor extension contains its own
formatting logic. The VS Code extension's inspections/refactorings/rename round-trip as LSP code actions
and requests with no logic duplicated locally; format-on-save matches `mwl fmt --check` byte-for-byte; the
Test Explorer runs `.mwlt` cases and shows coverage sourced from the Clover/lcov exporters; a captured
profile opens correctly in a speedscope-compatible viewer; a breakpoint set in VS Code's UI hits in
JIT-compiled code with correct variable values through the wired-up `mwl dap` adapter, with no
MWL-authored debugger UI code. The PhpStorm plugin registers `.mwl` as its own file type (opening one does
not invoke PhpStorm's bundled PHP support) and gets the same completion/hover/diagnostics/rename/formatting
round trip through the identical `mwl lsp`/`mwl fmt` binaries as VS Code — evidenced by both editors
agreeing byte-for-byte on the same file's formatted output and diagnostics — with breakpoints verified via
`mwl dap` directly, since PhpStorm's debugger UI is still not expected to exist yet.

### M11 — PHP transpiler (~10 weeks)
`mwl convert`: PHP source → AST → rewrite passes → idiomatic `.mwl` output. **This milestone is now on the
critical path for adoption rather than a convenience**, because PHP has no syntax for a `foreach` binding's
or a destructuring target's type and [ADR 0007](adr/0007-explicit-type-system.md) requires one for both: the
converter carries the type-inference engine MWL's compiler deliberately does not have for those two
positions, and writes the annotations into the output for a human to review. A plain local is now
mechanical instead — [ADR 0037](adr/0037-var-local-type-inference.md)'s `var $name = expr;` lets the
converter emit the initializer unchanged and let the compiler's own checker fix its type, no inference pass
needed. Where the remaining inference cannot decide, it emits `mixed` with a `TODO` naming the binding
rather than guessing — an honest `mixed` runs, and a wrong annotation would not. That inference pass is the
reason for the two extra weeks over the original estimate.

Mechanical rewrites where possible (`global` → parameter passing, function-scope `static` → a
`private static` property on the owning class or a parameter where there is no class, `static fn` → the
keyword dropped ([ADR 0008](adr/0008-static-and-global.md)), `extract()` → explicit assignment,
`settype()` → a second binding or an `as` conversion, every legacy `(int)`/`(string)`/… cast → the
equivalent `as` expression ([ADR 0034](adr/0034-legacy-cast-syntax-rejected.md)), flagged separately where
the source relied on PHP's silent `(int)"abc"` → `0` now that the rewritten `as` throws instead, simple
`$$var` → match on a map, `exec('php script.php …')` job dispatch →
`spawn script`, which is a real rewrite rather than a `TODO` because the isolation the original bought is
what the construct provides, a backed enum's case declarations and `->value` reads → an MWL `enum` and
an `as` conversion ([ADR 0010](adr/0010-enums-are-a-value-type.md)), and a call or reference to a PHP
built-in global function or constant (`strlen`, `array_map`, `PHP_EOL`, …) → the matching `Core`
class-and-member, `Core\Str::len`, `Core\Arr::map`, `Core\Env::EOL`, via a maintained PHP-name → `Core`
table that grows with the stdlib ([ADR 0011](adr/0011-functions-and-constants-are-class-members.md)),
`use Path\To\Name as Other;` → the local alias replaced with the real short name or the FQN at every use, a
stateless PHP trait (methods only) → an `interface` with the same method bodies as defaults plus plain
`implements` at every use site, and its `insteadof` conflicts → an explicit override calling the winner by
qualified name ([ADR 0043](adr/0043-interface-default-methods-and-delegation-replace-traits.md) §§ 6.1, 6.3,
superseding ADR 0015's now-withdrawn `TraitA::method as newName;` rewrite)); a
rewrite that needs a human look because it changes the shape of the surrounding code (a source file's own
top-level `function`/`const` declarations, with no built-in counterpart, are grouped into one generated
class named after the file — the same "needs a class to hang it on" shape the function-static rewrite
already has, per [ADR 0011](adr/0011-functions-and-constants-are-class-members.md); a stateful PHP trait
(declares a property) → an extracted interface plus a generated tracker class holding the state, with every
use site rewritten to `implements ... by $field` and a constructor assignment —
[ADR 0043](adr/0043-interface-default-methods-and-delegation-replace-traits.md) § 6.2, mechanical but flagged
for review); annotated `TODO`
diagnostics where neither applies (`eval` of constructed source, dynamic includes, unsupported `preg`
constructs, a `bindTo()` whose target closure never names `$this` — the one divergence ADR 0008 introduces,
and visible here rather than at run time — an enum that implements an interface or declares a method, which
has no mechanical destination under [ADR 0010](adr/0010-enums-are-a-value-type.md), a PHP trait's `static`
property or a trait method that calls back into an unrelated method of its host class, neither of which has
a mechanical destination
([ADR 0043](adr/0043-interface-default-methods-and-delegation-replace-traits.md) §§ 6.4-6.5), a class
declaring
`__get`/`__set` that needs a human call on whether the original logic was observation (→ `PropertyObserver`)
or computation (→ a per-property hook), a class declaring `__call`/`__callStatic` with no mechanical
destination at all ([ADR 0014](adr/0014-property-observer.md)), a `class_alias()` call whose target name is
computed dynamically or that exists only so two libraries can address one class under different names, with
no mechanical destination either ([ADR 0015](adr/0015-no-name-aliasing.md)), and C extensions).
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

**Verify:** macro benchmarks show a multiple over the baseline tier and over PHP 8.5 with JIT — measured per
[ADR 0026](adr/0026-performance-measurement-methodology.md)'s same-host PHP-oracle ratio, not a raw
cross-machine wall-clock claim; no correctness regressions in the full conformance suite when the
optimising tier is forced on.

### M13 — Optional FastCGI transport
Only if a deployment target requires it (shared hosting, IIS, an existing nginx estate). Implements the
same `Transport` trait as `mwl-http`, with a fuzzed record parser and `SCRIPT_FILENAME` handling that
resolves within a configured root — closing the historical vulnerability class by construction.

### M14 — Optional wasm32 browser target
Only if an embedding wants MWL running client-side in a browser tab, per
[ADR 0025](adr/0025-wasm-browser-target.md). A second codegen backend consuming the same M2 IR —
instruction selection to wasm32 opcodes via a pure-Rust emitter, not Cranelift, which has no wasm32
output. `spawn worker`/`spawn script`, coroutine-based suspension, and `.mwlx` extension loading are all
unavailable in this target (a diagnostic naming the ADR, never a silent no-op); every other language and
stdlib feature is unchanged. Adds `Core\Browser` as a seventh [ADR 0012](adr/0012-no-superglobals.md)
accessor domain (method table undesigned until this milestone starts), and requires `require` to resolve
every path at build time — no dynamic fallback, since there is no filesystem at runtime.

**Verify:** a `.mwl` file using none of the three excluded features compiles to a `.wasm` module and runs
identically to the native target's output on the same input, in a headless-browser test harness; a `.mwl`
file using `spawn`, suspension-requiring `Core` I/O, or a `.mwlx` extension for this target produces the
ADR-named diagnostic rather than a miscompile or a silent downgrade; `benches/abi-probe` gains a
browser-target guard for whatever cost claim this milestone's spike validates.

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
