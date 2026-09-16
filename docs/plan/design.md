# Novis — architecture and confirmed decisions

The frozen half of [the implementation plan](../implementation-plan.md): what was decided
before the first milestone, the architecture those decisions imply, and how the whole thing is
verified. Every decision here is argued out in [docs/adr/](../adr/README.md) — this states the
headline and links. The *live* half — where the work stands, and every milestone — is the plan
index, and one file per milestone beside this one.

---

## Context

**Novis** is the language this repository builds. The plan of record is
[docs/implementation-plan.md](../implementation-plan.md) — its status block is where the work stands,
and its table routes to one file per milestone. This document holds what that plan decided *before* the
first milestone; every decision it states is argued out in full in [docs/adr/](../adr/README.md).

The goal is a new programming language for web servers and CLI, written in Rust, that:

- takes PHP 8.5 syntax as its starting point so existing PHP projects can be migrated,
- compiles to native code just-in-time with no build step (edit file → run),
- has first-class in-language parallelism,
- can run another `.nvs` file as a fully isolated unit of work **inside the same process**, so a script
  never has to spawn an interpreter to get isolation (`rule:security/isolate-shares-nothing`),
- is memory-safe and hard to attack,
- serves HTTP from a **single process** with **no worker-pool ceiling** — fully isolated requests bounded
  only by a safety valve, never by a `pm.max_children` — while
  sharing one compiled-code cache across all of them,
- and is fast, safe and simple *first* — spending memory to stay that way rather than the reverse
  (`rule:programs/memory-priority`).

**Who this is for, and what it claims** — `rule:programs/audience` owns both
and this states only the headline. Novis is built to serve **web applications of every kind**, whatever the
application does and whoever wrote the code it runs; the safety properties are how it is built rather than a
segment's requirement, and the untrusted-code case is where they pay the most rather than what they are for.
Novis makes exactly three claims, and no incumbent language can add any of them later — **injection and
secret leakage are compile
errors**; **a request, a job, a connection and an untrusted script are each a budgeted isolate in one
process**; and **suspension has no colour**. Raw speed against PHP is measured
(`rule:testing/perf-two-mechanisms`) and is not the pitch: persistent-worker PHP
runtimes and PHP 8's JIT have answered enough of that argument that it no longer justifies a rewrite on its
own. The PHP-shaped syntax is an **on-ramp, never a compatibility promise**, and `rule:programs/no-compatibility-promise` forbids any
document from implying otherwise.

Intended outcome: a self-hosted toolchain (`nvs` binary) that runs `.nvs` files on the CLI, serves them
over HTTP from one process, ships a working framework and a supply-chain-safe package system
([0082](../decisions/0082.md), [0081](../decisions/0081.md)),
and can mechanically transpile an existing PHP codebase's *own* application code — including its `.phpt`
test suites — into Novis.

---

## Confirmed design decisions

Every row that names an ADR states only the headline — open that ADR for the mechanism, the exact
spellings rejected, and the reasoning. Do not restate that detail here when adding a row.

| Area | Decision |
|---|---|
| Implementation language | Rust (stable, pinned via `rust-toolchain.toml`) |
| Resource priorities | Security → semantics → latency → simplicity → memory footprint, in that order, within an enforced per-request cap (`rule:programs/memory-priority`) |
| Execution | Cranelift JIT from day one, no interpreter tier; baseline codegen first, optimising tier later |
| Code cache | Content-addressed on-disk cache (BLAKE3) + in-process `Arc` sharing; hot-reloads on an edit via a per-path pointer swap, no watcher, no restart (`rule:config/an-edit-reaches-the-next-request-without-a-restart`); the on-disk file format, its mmap-verify-then-execute read path and its eviction policy are `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` |
| Parallelism | Hybrid: `async`/`await` for I/O inside a task (same heap, cooperative) + isolated workers on other cores for CPU work |
| Suspension | Stackful coroutines — no async colouring; any function may yield |
| Isolated execution | `spawn script 'file.nvs'` runs another file in-process as a child isolate, file-only, never a source string (`rule:security/isolate-shares-nothing`) |
| Type system | Static, mandatory, explicit; every binding's declared type never changes; `uint` alongside signed `int` (`rule:types/declaration`) |
| Enums | A closed, named integer type, C#-style; PHP's class-like enum design (`::cases()`, methods, `string` backing) is disregarded entirely (`rule:enums/closed-integer-type`) |
| Scoping and state | `static` is a class-member modifier only; no function-scope `static`, no `static fn`, no `global` (`rule:statements/static-is-a-member-modifier`) |
| No superglobals | No variable is ever populated by the host; every PHP superglobal becomes a `Core` accessor class, and `$GLOBALS`/`$_REQUEST` have no replacement (`rule:statements/no-host-populated-variables`) |
| Object comparison | Ordering two objects requires the global `Comparable` interface; PHP's ambient property-walk fallback is rejected outright (`rule:classes/comparable`) |
| Property access | A property's own hook runs first, then a declared `PropertyObserver` second, always both, never a fallback for a missing property (`rule:classes/property-observer`) |
| OOP-only: no free functions, no global constants | Every callable is a method, every constant a class constant; built-ins live under `Core` domain classes (`rule:classes/no-free-functions-or-constants`) |
| Name aliasing | No `class_alias` or import `as`; a compile-time-only `type` alias for a type expression is the one exception (`rule:statements/nothing-gets-a-second-name`) |
| Code reuse | No `trait`; shared behavior is a `public`/`private` interface method body, shared state is explicit `implements Interface by $field;` delegation, and any resulting name collision is always a compile error requiring an explicit override — there is no `insteadof` (`rule:classes/no-traits`) |
| PHP compatibility | Pragmatic superset of the syntax, not of the type discipline: PHP 8.5 syntax accepted, `strict_types` implicit, no `eval`/`$$var`/`goto`/`extract()`/`settype()`/pipe operator (`\|>`, deliberately unparsed — see `nvs-syntax`'s module docs). Existing PHP does not run unconverted — see *Consequences to accept* below, and each ADR above for its own divergence from PHP |
| Templating | `<?nvs … ?>` inline-HTML mode, `<?= ?>` short echo, `.nvs` extension. Explicit escaping (not auto) |
| Request state | Strict shared-nothing: only compiled code survives a request; no connection pooling in v1 (seam reserved). A request is the root isolate of a tree; `spawn script` adds children to it |
| Regex | Pure Rust two-tier: `regex` (linear-time) → `fancy-regex` (lookaround/backrefs) fallback |
| Security | Server-level `nvs.toml`, root-owned, TOML (`rule:config/the-file-is-nvs-toml-and-it-is-toml`), deny-by-default capabilities + hard per-request limits (`rule:config/three-changeability-classes`) |
| Serving | Built-in **HTTP/1.1** server, scoped to a development server and a proxied origin; no TLS listener, no h2c, no HTTP/3, no FastCGI, no compression (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`) |
| Text and binary | `string` is guaranteed-valid UTF-8 and counts extended grapheme clusters; binary data is the separate `bytes` primitive, counting bytes (`rule:types/bytes`) |
| Databases | One `Core\Db` API over MySQL, MariaDB (a driver of its own, not a MySQL version), PostgreSQL, SQLite and MS SQL Server: connections named in root-owned config, every statement prepared, a transaction is a closure (`rule:core-classes/db-one-api`) |
| Tooling | LSP + formatter, test runner, debugger + profiler, package manager |
| Audience | Web applications of every kind; the pitch is isolation and qualifiers, and PHP syntax is an on-ramp rather than a compatibility promise (`rule:programs/audience`) |
| Packages | Content-addressed source archives from a first-party registry or (root-only) a git URL, resolved by minimal version selection, with no package code running before the program and capabilities granted per package (`rule:packaging/a-package-is-its-digest`) |
| Framework | First-party and split by `rule:core-api/tier-placement`'s six tests: privileged halves in `Core`, the opinionated layer as the `nvs/web` package; no ORM, no runtime container, the language is the view layer (`rule:programs/first-party-framework`) |
| Real-time | WebSocket and SSE connections are root isolates opened the way a script is spawned; fan-out is a bounded `Core\Topic` (`rule:concurrency/a-connection-is-a-root-isolate`) |
| Background work | A durable job is a row in a `Core\Db` table, enqueued inside the caller's transaction and run as an isolate (`rule:concurrency/enqueue-commits-with-your-write`) |
| API contracts | OpenAPI 3.1 generated while compiling from the route table and derived codecs, with `nvs api diff` as a breaking-change gate (`rule:routing/api-document-is-generated-from-the-route-table`) |
| Testing | Hand-written suite is normative; `.phpt → .nvst` transpiler imports PHP's corpus |
| Migration | `nvs convert` — real PHP→Novis transpiler |
| Extensions | Three tiers: built-in, sandboxed **WebAssembly components** (`.nvsx`), statically linked native. No `dlopen` (`rule:packaging/an-extension-is-a-sandboxed-wasm-component`) |
| Platforms | Windows x86_64, Linux x86_64, macOS (x86_64 + aarch64) |
| Licence | MIT |

### Rationale for the two calls left open

**Built-in HTTP server over FastCGI.** FastCGI's worst vulnerability class — the
`SCRIPT_FILENAME`/`PATH_INFO`/`cgi.fix_pathinfo` RCE family — exists *because* the decision of which file
to execute is **derived from the URL by one process and trusted by another**. A native server keeps that
derivation from happening at all, which is
`rule:http-server/a-path-is-never-derived-from-a-url`'s governing rule. Throughput over
loopback/UDS differs by single-digit microseconds per request, irrelevant beside script execution. A bespoke
FCGI record parser would be attack surface we own; `hyper` is memory-safe and among the most-fuzzed HTTP
stacks in existence. FastCGI is closed rather than deferred: HTTP over a Unix socket already serves every
deployment it was reserved for.

**HTTP/1.1 only.** nginx `proxy_pass` speaks HTTP/1.1 upstream, so h1 keep-alive is mandatory either way.
h2c is not taken even though `hyper` would give it nearly free: a multiplexing proxy concentrates many
streams onto one connection and therefore onto **one core**, which defeats the connection-level balancing
thread-per-core depends on and cannot be rebalanced, since a request never migrates. It also buys the Rapid
Reset and `CONTINUATION`-flood classes. No production proxy speaks HTTP/3 to an origin; the edge terminates
QUIC and talks h1 upstream → h3 is pure cost. TLS is terminated by the proxy, so there is no inbound
listener for it (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`).

### Consequences to accept

- **`Hello World` is ~3–5 weeks out, not day one.** With no interpreter tier, the first program requires
  the whole front end *plus* a working native backend. Mitigation: the backend ships as a *baseline* tier
  where every operation lowers to a call into a Rust runtime helper — mechanically close to an interpreter
  loop, so it is fast to get correct, and typed inlining layers on afterwards without redesign.
- **Database connections are pooled per core, so a connection reset is a security boundary.** Shared-nothing
  is a rule about *program* state; a connection is host state Novis code cannot observe, so pooling it costs
  the model nothing (`rule:security/db-pool-reset-is-a-boundary`). What it does cost is a reset that must be
  provable rather than best-effort — a connection that cannot be proven clean is destroyed, because one
  tenant's session state arriving in another tenant's request is a leak, not a performance bug.
- **An existing PHP application's framework and packages do not come along.** No trait, no `__call`, no
  `ArrayAccess`, no runtime autoloader ([ADRs 0043](../decisions/0043.md),
  [0014](../decisions/0014.md), [0053](../decisions/0053.md),
  [0061](../decisions/0061.md)) means the ecosystem built on those is
  unreachable at any price — so `nvs convert` (M11) ports an application's own code onto Novis's own
  framework, and never onto its old one. `rule:programs/audience` records
  why this is survivable and what the alternative cost.
- **A JIT means a native-codegen component in the trusted core.** User programs stay fully memory-safe
  (all codegen is type-checked and bounds-checked); the codegen itself, the coroutine stack switcher and
  the arena are the audited unsafe surface. See "Unsafe policy".
- **Resident memory is higher than a footprint-tuned runtime's, and scales with in-flight concurrency
  rather than request rate.** 64 KiB of reserved stack per task, 16-byte values, copy-on-write clones and a
  per-request arena held to its peak are all deliberate purchases of safety, speed or simplicity. Sizing a
  deployment therefore means sizing for concurrency — tasks × stack, plus concurrent requests × their
  `memory` cap — and deployment docs must say so rather than quote a typical RSS.
- **Existing PHP does not run unconverted.** PHP has no syntax for the type of a `foreach` binding or a
  destructuring target (`rule:types/declaration`), and every one of its global
  functions and global constants needs a new home on a class before it type-checks at all
  (`rule:classes/no-free-functions-or-constants`). A plain local has a type-eliding
  spelling now — `rule:types/var-inference`'s `var` — so `nvs convert` can emit that
  directly instead of inferring and writing an annotation, but it still has to write annotations and
  rewrite call sites for everything else, not just drop `.php` files into a document root. M11 stays
  mandatory rather than a convenience, and the imported `.phpt` pass rate is still structurally lower than
  a compatibility-first design's, not through bugs.
- **One new language construct that PHP has no equivalent of.** `spawn script` is a surface a developer
  has to learn and the spec has to define next to `require`, which they will confuse it with. Accepted:
  the requirement it answers — run another file, isolated, without a second process — has no other honest
  answer, and it reuses the request boundary and the worker value rules rather than adding either
  (`rule:security/isolate-shares-nothing`).
- **This is an 18–24 month effort at full-time pace for one experienced engineer**, front-loaded: M0–M4
  (a usable CLI language) is roughly 3–4 months; the stdlib and DB drivers are the long tail.

---

## Validated premises

The M0 spikes are now permanent guard tests in [`benches/abi-probe`](../../benches/abi-probe/), because
several decisions here rest on how Cranelift, `corosensei` and Wasmtime behave rather than on Novis's own
code, and a dependency bump can invalidate them silently. **The tests own the numbers; this document does
not restate them.**

| Premise | Guarded by | Argued in |
|---|---|---|
| Native unwinding through JIT frames is unavailable on every platform — the premise the calling convention exists for | `tests/unwind_unavailable.rs` | [0002](../decisions/0002.md) |
| A throw propagates, and a runtime panic is *contained*, across native frames | `tests/invariants.rs` | [0002](../decisions/0002.md) |
| A checked-return frame stays cheap, and throwing costs no more than returning | `a_checked_return_frame_stays_cheap`, `throwing_costs_about_the_same_as_returning` | [0002](../decisions/0002.md) |
| A coroutine can suspend from beneath live JIT frames, cheaply | `a_helper_can_suspend_with_jit_frames_live_above_it`, `a_coroutine_round_trip_stays_cheap` | [adr/README.md](../adr/README.md), *Stackful coroutines* |
| A wasm guest cannot read past the host heap or outlive its deadline, and the boundary is affordable | `tests/wasm_sandbox.rs`, `a_host_to_guest_call_stays_cheap`, `per_request_instantiation_stays_affordable` | [0003](../decisions/0003.md) |
| An OS process still costs orders of magnitude more than a task — the whole cost case for in-process isolation | `an_os_process_costs_orders_of_magnitude_more_than_a_task` | [0006](../decisions/0006.md) |

Those spikes forced one design change, and it is normative for everything below: **exceptions propagate by
checked return, not by unwinding**, and every runtime helper is `extern "C"` wrapping `catch_unwind`. The
signature, the measured cost and the reasoning are in `rule:errors/propagation`.

## Extension system

Three tiers, each the right answer for a different class of code rather than a compromise. The full
reasoning — the WIT interface, the handle-table value model, the measured boundary costs, the isolation and
loading rules, and the decisive rejection of `dlopen` — is in `rule:packaging/an-extension-is-a-sandboxed-wasm-component`.

- **Tier 0 — built-in (`nvs-stdlib`).** Compiled into the binary, native, direct heap access, no boundary.
  Home of the fine-grained primitives whose total cost is comparable to a call.
- **Tier 1 — WebAssembly component extensions (`.nvsx`).** The default for third parties: one binary that
  runs on every platform, sandboxed by construction, authorable in any language `wit-bindgen` targets.
- **Tier 2 — statically linked native.** A Rust crate compiled into a custom `nvs` binary, for first-party
  subsystems needing raw sockets, TLS or direct heap access — `nvs-db`, `nvs-regex`, crypto. It requires
  building from source, which is the right friction for code that runs unsandboxed.

One sequencing constraint, which is why this section is in the plan at all: the `nvs:ext@1.0.0` WIT world
must be authored **during** the stdlib milestone (M8), from the same value-access design as the built-ins,
so that the Tier 0 internal interface and the Tier 1 guest interface are one design rather than two that
drift. Deferring it to M9 would mean retrofitting.

## Architecture

```
                       .nvs source
                            │
    ┌───────────────────────▼───────────────────────┐
    │ nvs-syntax    lexer (dual mode, inline HTML)  │
    │               parser → AST + spans            │
    ├───────────────────────────────────────────────┤
    │ nvs-hir       name resolution, namespaces,    │
    │               class graph, symbol table       │
    ├───────────────────────────────────────────────┤
    │ nvs-types     declared types, unions, flow    │
    │               narrowing, no inference engine  │
    ├───────────────────────────────────────────────┤
    │ nvs-ir        CFG/SSA, safepoints, refcount   │
    │               ops, optimisation passes        │
    ├───────────────────────────────────────────────┤
    │ nvs-codegen   Cranelift → native code (W^X),  │
    │               status checks, helper calls     │
    └───────────────────────┬───────────────────────┘
                            │  Arc<CompiledUnit>  (immutable, shared)
    ┌───────────────────────▼───────────────────────┐
    │ nvs-host   unit cache (single-flight compile) │
    │            isolate = arena + globals + limits │
    │            (a request is one isolate's root)  │
    └───────┬───────────────────────────────┬───────┘
            │                               │
    ┌───────▼───────┐               ┌───────▼───────┐
    │ nvs-http      │               │ nvs-cli       │
    │ h1, TCP + UDS │               │ run / test    │
    └───────────────┘               └───────────────┘
       one handle(Request) -> Response seam; `rule:testing/test-attribute`'s
       synthetic request and #[Test(server: true)] are its
       other callers. No transport trait, no FCGI.
```

### Thread-per-core, shared-nothing runtime

The decisive structural choice. One single-threaded runtime **pinned per CPU core**; connections are
load-balanced across cores; a request never migrates between cores.

**That runtime is ours, and it is not `async`.** It is `corosensei` stackful coroutines on a thread-per-core
scheduler of our own, and no crate of ours depends on `tokio` — where it appears at all it is compiled
with `sync` alone, a channel library and not a runtime (`rule:concurrency/one-scheduler`).
Earlier drafts of this section and of `rule:http-server/a-core-is-never-blocked-on-a-syscall`
 said "Tokio"; that was stale text rather than a live decision, and both now say the same thing.

**A socket is therefore a synchronous one that parks its coroutine.** `nvs-host` exposes a stream
implementing plain `std::io::Read` and `Write` whose `read` returns `WouldBlock` to the reactor, parks the
coroutine and resumes when the descriptor is ready — so it *blocks* the request and never the core. That is
what lets every synchronous Rust crate compose with no async at all: `rustls` over it, `httparse` inside
`hyper`, the wire codecs each database driver uses. The one place a `Future` is polled is a `block_on` over
`hyper`'s h1 connection future (M7): h1 needs no `Executor` and spawns nothing, so polling one future per
connection on the coroutine that owns it is a loop and a waker, not a second scheduler. `hyper` with
`default-features = false` brings no `tokio`, and the reason it is used at all rather than hand-rolled is
the one below — request framing is a security-critical parser.

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
decimal | bytes`, beside the never-written storage state a property slot carries
(`rule:classes/an-unwritten-property-read-throws`). There is no `closure` or `resource` tag: a closure is an
ordinary object (`rule:types/callable-is-a-closure`) and an engine-owned handle is a `Core` class holding a
key into its own context's table, so neither buys a second heap shape or a second release path.
`uint` is a tag, not a wider slot, so it costs nothing here; the type system
that demands it is `rule:types/declaration`, which also owns the array element-type stamp
carried on the array header. `decimal` is the one tag whose value does not fit the payload alone — its 96-bit
mantissa spends the padding bytes too, so it is the whole sixteen — and `nvs_runtime::decimal`'s own module
doc is the home for that layout.

Memory: refcounting + copy-on-write arrays/strings (PHP semantics). Reference cycles — objects only: a
string is immutable and an array copies on write, so neither can close one — are reclaimed by the teardown
sweep of `rule:security/isolate-teardown-is-a-drain-then-a-sweep`, which is what bounds a
cycle by the request or isolate that built it. Long-running CLI scripts additionally get an optional
mark-sweep cycle collector running at safepoints — a decision still open; whichever milestone implements
its run routine also gives it a `gc`-kind trace event, at no cost to the safepoint poll itself
(`rule:observability/trace-events-carry-a-kind`).

Both choices here — 16 bytes per value instead of 8, and peak-not-average retention inside a request — cost
memory to buy correct PHP semantics and a collector that never runs on the request path. That is the
priority ordering in `rule:programs/memory-priority`, not an oversight to optimise away later.

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
// DashMap<UnitKey { path, content_hash, env_hash }, CompileState>
```

The first requester inserts `Compiling` and compiles on a **dedicated compile pool** (never on a
request-serving core, so compilation cannot stall request handling). Concurrent requesters await the same
broadcast — N simultaneous first-hits compile exactly once, and none of them block a core. Staleness:
`stat` (mtime+size) → BLAKE3 content hash → atomic swap. Governed by
`opcache.validate = never|mtime|hash`. Native pages are mapped `RX`, never `RWX` (W^X discipline).

Because the key above is not `{path}`, a `Ready` entry is write-once — two versions
of a file are two entries, never one overwritten. A small separate index, `path → current content_hash`,
sits in front of it and is the one thing a hot-reload actually swaps; `rule:config/an-edit-reaches-the-next-request-without-a-restart`
holds the only copy of that mechanism, why revalidation needs no filesystem watcher, and how a file edited
under a live server reaches the next request with no restart while a request already running keeps the
version it started with. The third key field, `env_hash`, is the environment a unit was compiled in —
including the loaded extension set — so a config reload that changes that set turns every unit into an
ordinary cache miss rather than needing an invalidation pass
(`rule:config/the-config-is-an-immutable-snapshot`).

### Per-request isolation

Each request gets: its own heap arena with a hard byte cap; fresh backing state for the `Core\Request`/
`Core\Server`/`Core\Session` accessors (`rule:statements/no-host-populated-variables`, replacing PHP's
superglobals); a copy-on-write overlay of the config; its own coroutine tree. At request end the arena is
released
wholesale. `catch_unwind` at the request boundary means a runtime panic kills one request, never the
process, which is why `panic = "unwind"` is load-bearing in every profile. Every capability check
consults the *request's* config snapshot, so a script cannot affect its neighbours.

### In-process isolated script execution

The provisional surface, the semantics, and what is/is not shared are decided and stated in full in
`rule:security/isolate-shares-nothing` — including the `spawn script … with(…)` example, how
values cross (the graph-copy operation `rule:classes/two-copy-depths` now
formally defines, shared with `serialize()`/`unserialize()`), how budgets are accounted (at the root of the
request tree, never per isolate), the `script.spawn` capability and its path resolution, and failure
arriving as a value rather than as an unwind. The spec pins the exact grammar down in M5. The three
invariants no optimisation may trade away are listed in [AGENTS.md](../../AGENTS.md).

The structural consequence for this plan: `nvs-host` gains **one** `Isolate` type, and an inbound HTTP
request *is* the root isolate of its tree. The server path (M7) and the `spawn script` path (M5) therefore
share one arena setup, one teardown, one place limits are enforced — and one state-bleed test suite. That
is why isolates land in M5, before the server that depends on them.

### `nvs.toml` — server-level, root-owned

A directive registry in a root-owned TOML file, with per-app capability blocks living in the *root* config so
an application can never grant itself rights. `nvs.toml` states **defaults, not ceilings**: a directive is a
limit that cannot be exceeded only when it cannot be changed at runtime at all. Each directive carries one
of three changeability classes — `System`, `Runtime`, `RuntimeTighten`.

`rule:config/three-changeability-classes` holds the only copy of the directive layout: the classes and
why each is argued per directive, the `[core]`, `[limits]`, `[limits.hard]`, `[capabilities]` and `[app]`
tables, and the `Core\Config::set`/`::get`/`::restore` overlay rules.
`rule:config/the-file-is-nvs-toml-and-it-is-toml` holds the only copy of why the file is TOML rather than
INI, and of the `Core\Config` signatures. Do not restate either here.

Two cross-cutting consequences the milestones below depend on:

- A widened limit lives on the request-local copy-on-write overlay, so it dies with the request that set it
  and is never visible to another. A set refused by a ceiling or a class returns `false` and leaves the
  value unchanged — it is **not** clamped.
- Every `[limits]` value is accounted against the **root of a request tree**, not per isolate, which is
  what keeps the process worst case independent of how many isolates a script creates
  (`rule:security/isolate-shares-nothing`).

---

## Repository layout

The layout, and which crates exist today versus which are deferred:
[README](../../README.md#repository-layout). Crates for later milestones are created when their milestone
starts rather than sitting empty.

### Unsafe policy

`unsafe_code = "forbid"` workspace-wide, in [Cargo.toml](../../Cargo.toml)'s `[workspace.lints]`.
`forbid` is not something an individual `#[allow]` can relax, so a crate that keeps it cannot hold an
`unsafe` block at all — which makes the roster of crates that opt down to `deny` the whole policy.

**That roster is `tools/lints.py`'s `UNSAFE_CRATES`, with the reason each crate needs it, and this
section does not restate it.** A prose copy here is what went stale: it named four crates while seven
carried `unsafe`, and nothing failed. Cargo refuses a manifest that inherits `[workspace.lints]` and
overrides one entry, so those crates must restate the whole table; `tools/lints.py` generates the
copies and `--check` fails the `lint` CI job and `verify.py` on any drift. A crate joins the roster
with an ADR.

Each `unsafe` block carries its own `#[allow(unsafe_code, reason = "...")]`, plus a safety-invariant
doc comment and dedicated Miri/ASAN coverage. `unsafe_op_in_unsafe_fn` is edition 2024's
warn-by-default and is promoted by the `-D warnings` both clippy gates run with, rather than being
set per crate. Prefer `corosensei` (audited, handles Windows SEH and aarch64) over a hand-rolled
switcher.

---

---

## Overall verification strategy

- **Unit** — `cargo test` per crate; `insta` snapshots for AST/IR/codegen.
- **Conformance** — hand-written `.nvst` suite as the normative definition of Novis; imported `.phpt`
  corpus tracked as a compatibility percentage.
- **Property/fuzz** — `proptest` for the array and string implementations; `cargo-fuzz` on lexer, parser,
  HTTP parser, multipart, regex and JSON, run continuously in CI.
- **Sanitisers** — Miri on the safe subset, ASAN/TSAN on the unsafe core and the scheduler.
- **Security** — the adversarial suites from M6/M7 (capability escape, resource exhaustion, cross-request
  *and* cross-isolate state bleed, execution-root escape, traversal, smuggling) plus `cargo deny` advisories
  on every build.
- **Performance** — `criterion` microbenchmarks and application-level macro benchmarks, always compared
  against the locally installed PHP 8.5.8, with results committed so regressions are visible in diffs.
