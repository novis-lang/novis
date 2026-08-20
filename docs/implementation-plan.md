# MWL — Modern Web Lang: Implementation Plan

## Context

This is the living implementation plan for **MWL**, the language this repository builds. It is the
single place where the plan of record lives; the decisions it summarises are argued out in full in
[docs/adr/](adr/README.md). Keep it in sync as milestones land.

The goal is a new programming language for web servers and CLI, written in Rust, that:

- takes PHP 8.5 syntax as its starting point so existing PHP projects can be migrated,
- compiles to native code just-in-time with no build step (edit file → run),
- has first-class in-language parallelism,
- is memory-safe and hard to attack,
- serves HTTP from a **single process** handling unlimited concurrent, fully isolated requests while
  sharing one compiled-code cache across all of them,
- and is fast, safe and simple *first* — spending memory to stay that way rather than the reverse
  ([ADR 0004](adr/0004-memory-for-simplicity.md)).

The motivation is the structural ceiling of PHP itself: process-per-request or worker-pool models, no
in-language parallelism, a C runtime with a long CVE history, and a stdlib whose semantics block
optimisation. MWL keeps PHP's authoring experience (no build step, inline templating, familiar syntax)
and replaces the execution model underneath it.

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
| PHP compatibility | **Pragmatic superset.** PHP 8.5 syntax accepted; `strict_types` implicit; no `eval`, `$$var`, `goto`, `global`, `extract()` |
| Templating | `<?mwl … ?>` inline-HTML mode, `<?= ?>` short echo, `.mwl` extension. Explicit escaping (not auto) |
| Request state | **Strict shared-nothing.** Only compiled code survives a request. No connection pooling in v1 (seam reserved) |
| Regex | Pure Rust two-tier: `regex` (linear-time) → `fancy-regex` (lookaround/backrefs) fallback |
| Security | Server-level `mwl.ini`, root-owned, php.ini-style, deny-by-default capabilities + hard per-request limits; per-directive changeability so scripts may tighten but never widen |
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
- **This is an 18–24 month effort at full-time pace for one experienced engineer**, front-loaded: M0–M4
  (a usable CLI language) is roughly 3–4 months; the stdlib and DB drivers are the long tail.

---

## Spike results (2026-08-20) — one design change forced

Rust 1.97.1 + Cranelift 0.128.4 on `x86_64-pc-windows-msvc`. Spikes live in the scratchpad
(`clif-spike/src/{main,bin/abi,bin/coro}.rs`).

**Finding 1 — native unwinding is unavailable, on every platform.** A Rust panic raised in a runtime
helper called from JIT code escaped as SEH `0xE06D7363` and killed the process; `catch_unwind` above the
JIT frame never saw it. Cause confirmed in the crate source: `cranelift-jit` never calls
`RtlAddFunctionTable` (Windows) or `__register_frame` (ELF). Its only unwind support sits behind the
`wasmtime-unwinder` feature, which is Wasmtime's *private* side-table unwinder, not the platform unwinder
that Rust panics and C++ exceptions use. Setting `unwind_info = true` makes Cranelift *emit* unwind data
that nothing registers with the OS.

**Design change — exceptions propagate by checked return, not by unwinding.** Normative ABI for every
compiled MWL function and every runtime helper:

```rust
extern "C" fn(*mut Ctx, *const Value, *mut Value) -> i32   // 0 = ok, non-zero = throw pending in Ctx
```

Codegen emits `icmp_imm ne 0` + `brif` after every call; the error block drops this frame's locals and
returns the status onward. **Measured cost: 1.3 ns per frame** (3.9 ns for a 3-frame chain, 5M
iterations). This is strictly better than native unwinding for MWL: no platform-specific unwind
registration on 3 platforms × 2 architectures, nothing to unwind across a coroutine stack switch, and
error-path refcount drops become explicit IR the optimiser can see instead of opaque landing pads.

**Corollary — helper ABI is `extern "C"`, never `extern "C-unwind"`, and every helper wraps its body in
`catch_unwind`** (one macro, zero happy-path cost) to convert a runtime panic into a `FATAL` status. This
is what preserves per-request isolation without unwind tables, and it makes `panic = "unwind"` load-bearing
rather than merely preferred. A custom panic hook must route the message to the request log instead of
stderr.

**Finding 2 — stackful coroutines work under JIT frames.** `corosensei` 0.2.2 suspends from a helper with
two JIT frames live above it and resumes correctly back into them; repeated switches leave frames intact;
checked-return throws and contained panics both still propagate correctly inside a coroutine. **Measured:
25 ns per suspend/resume round trip.** The "no async colouring" promise holds — any MWL function can do
I/O without being marked `async`.

**Finding 3 — the toolchain is in place.** Rust 1.97.1 stable, MSVC 14.44 + Windows SDK 10.0.26100 for
linking, PHP 8.5.8 available as a comparison oracle.

**Finding 4 — WebAssembly is viable as the extension mechanism.** wasmtime 41 coexists with cranelift
0.128 (no dependency conflict, since wasmtime is built on it). Measured: host→guest call 11.5 ns,
guest→host accessor call 9.0 ns, 1 KiB bulk copy into guest memory 11.7 ns, fresh pooled instance plus one
call 7.57 µs, and epoch interruption correctly traps a deliberately infinite guest loop. Against a 1.3 ns
built-in call frame, an extension call costs ~10 ns more — noise for coarse-grained work, significant for
fine-grained work, which is what the tier split below is for.

## Extension system

Full reasoning in `docs/adr/0003-extension-system.md`. Three tiers, each the right answer for a different
class of code rather than a compromise:

**Tier 0 — built-in (`mwl-stdlib`).** Compiled into the binary. Native, direct heap access, no boundary.
Home of the fine-grained primitives: string and array functions, conversions, anything whose total cost is
comparable to a call.

**Tier 1 — WebAssembly component extensions (`.mwlx`).** The default for third parties. A `.mwlx` is a
wasm component with an `mwl.manifest` custom section — one file, one binary, every platform. Sandboxed and
memory-safe by construction, so a crashing or hostile extension harms one request rather than the process.
Interface declared in WIT (`mwl:ext@1.0.0`); `wit-bindgen` generates bindings for Rust, C, C++, Zig, Go,
JS and Python, so authors are not restricted to C the way PHP's are. Semantic versioning is part of the
contract, which fixes PHP's recompile-every-minor-release problem.

**Tier 2 — statically linked native extensions.** A Rust crate compiled into a custom `mwl` binary, for
first-party subsystems needing raw sockets, TLS or direct heap access — `mwl-db`, `mwl-regex`, crypto.
Native speed, safe because it is safe Rust, and it requires building from source, which is the right
friction for code that runs unsandboxed.

**Explicitly rejected: `dlopen` of native shared libraries** — the PHP/Python/Node model. It destroys
memory safety (the central product claim), destroys request isolation (one segfault kills every in-flight
request in a single-process server), makes every `mwl.ini` capability grant advisory rather than enforced,
and cannot ship one precompiled binary per platform anyway — Rust has no stable ABI, so an extension would
also have to match the host's compiler version. It fails both of the requirements that motivated asking.

Key mechanics:

- **Values cross as handles, never pointers.** MWL values stay in the host heap; the guest gets an opaque
  `value` resource — a bounds-checked index into a per-call handle table — and reads through host accessors
  (9.0 ns). The guest cannot forge a host pointer, the host stays authoritative for refcount and COW, and
  large arrays are never copied wholesale.
- **Statically typed.** The manifest declares functions, classes, constants and wanted `mwl.ini`
  directives; the host registers them into the compiler's symbol table, so `mwl check` type-checks calls
  into extensions and codegen emits a direct trampoline call rather than dynamic dispatch.
- **Root-controlled loading.** `extension = image.mwlx` in the root-owned `mwl.ini`, with optional hash
  pinning and signature verification. A project cannot cause code to be loaded.
- **Fresh instance per request, created lazily.** Extension state cannot leak between requests — something
  PHP cannot offer. Paid only for extensions a request actually calls, so realistically 8–23 µs.
- **No ambient authority.** No WASI by default; the guest gets only MWL's capability-checked host
  functions, so extension I/O obeys the same root config as script code. WASI is an opt-in world whose
  preopens derive from the capability grants.
- **Epoch interruption** ties guest execution to the per-request CPU cap, so a runaway extension traps
  instead of hanging a core.
- **Async composes for free** — wasmtime's async support is stack switching, the same mechanism as MWL's
  coroutines, so an extension doing I/O suspends the request like any other function.

One sequencing constraint: the WIT world must be authored **during** the stdlib milestone, from the same
value-access design, so the internal Tier 0 interface and the Tier 1 WIT world are the same shape rather
than two designs that drift apart.

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
    │ mwl-types     gradual typing, inference,      │
    │               declared-type checking          │
    ├───────────────────────────────────────────────┤
    │ mwl-ir        CFG/SSA, safepoints, refcount   │
    │               ops, optimisation passes        │
    ├───────────────────────────────────────────────┤
    │ mwl-codegen   Cranelift → native code (W^X),  │
    │               unwind tables, helper calls     │
    └───────────────────────┬───────────────────────┘
                            │  Arc<CompiledUnit>  (immutable, shared)
    ┌───────────────────────▼───────────────────────┐
    │ mwl-host   unit cache (single-flight compile) │
    │            per-request isolation + limits     │
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
require full-range `i64`. Types: `null | bool | int(i64) | float(f64) | string | array | object | closure |
resource`.

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

Each request gets: its own heap arena with a hard byte cap; a fresh set of superglobals; a
copy-on-write overlay of the config; its own coroutine tree. At request end the arena is released
wholesale. `catch_unwind` at the request boundary means a runtime panic kills one request, never the
process (`panic = "unwind"` in release for this reason). Every capability check consults the
*request's* config snapshot, so a script cannot affect its neighbours.

### `mwl.ini` — server-level, root-owned

php.ini-style directive registry. Each directive carries a changeability class:

- `System` — boot only, immutable at runtime,
- `RuntimeTighten` — a script may narrow it, never widen it (all capabilities, memory ceiling),
- `Runtime` — freely settable per request, discarded at request end.

```ini
[core]
opcache.validate       = hash
cache.dir              = /var/cache/mwl        ; refuses to start if world-writable

[limits]                                        ; RuntimeTighten
memory                 = 128M
cpu_time               = 5s
wall_time              = 30s
max_tasks              = 64
max_output             = 32M

[capabilities]                                  ; deny-by-default, RuntimeTighten
fs.read                = /srv/www:/srv/shared
fs.write               = /srv/www/var
net.out                = api.stripe.com:443
process.exec           = off
env.read               = APP_ENV

[app "shop"]                                    ; per-app blocks live in the ROOT config,
root                   = /srv/www/shop          ; so an app can never grant itself rights
capabilities.fs.write  = /srv/www/shop/var
```

`ini_set()`/`ini_get()` operate on the request-local overlay under those rules.

---

## Repository layout

```
d:\swlang\
├─ Cargo.toml                 # workspace
├─ rust-toolchain.toml        # pinned stable + rustfmt, clippy, llvm-tools
├─ deny.toml                  # cargo-deny: advisories, licences, bans
├─ .github/workflows/ci.yml   # win/linux/macos × test, clippy -D warnings, fmt, deny, miri
├─ docs/
│  ├─ spec/                   # normative language reference (grammar, semantics)
│  ├─ adr/                    # architecture decision records (one per table row above)
│  └─ threat-model.md
├─ crates/
│  ├─ mwl-diagnostics/        # spans, source maps, rendering (terminal + JSON for LSP)
│  ├─ mwl-syntax/             # lexer (MWL + PHP mode, inline HTML), parser, AST
│  ├─ mwl-hir/                # resolution, namespaces, class graph, symbols
│  ├─ mwl-types/              # gradual type system, inference, checking
│  ├─ mwl-ir/                 # CFG/SSA IR, safepoints, refcount ops, passes
│  ├─ mwl-codegen/            # Cranelift backend, unwind info, W^X, helper table
│  ├─ mwl-runtime/            # values, arrays, strings, objects, coroutines,
│  │                          #   scheduler, arena, limits   (holds the unsafe core)
│  ├─ mwl-stdlib/             # native builtins
│  ├─ mwl-regex/              # two-tier engine + preg_* layer          [tier 2]
│  ├─ mwl-db/                 # driver trait + mysql / pgsql / sqlite / mssql [tier 2]
│  ├─ mwl-ext/                # .mwlx loader, WIT host impl, handle tables,
│  │                          #   per-request instancing, capability bridge
│  ├─ mwl-config/             # mwl.ini registry, changeability classes, overlays
│  ├─ mwl-cache/              # content-addressed artifact cache
│  ├─ mwl-host/              # Transport trait, unit cache, request isolation
│  ├─ mwl-http/               # hyper h1 + h2c transport, optional rustls
│  ├─ mwl-fcgi/               # (M12) optional FastCGI transport
│  ├─ mwl-test/               # .mwlt runner
│  ├─ mwl-fmt/                # formatter
│  ├─ mwl-lsp/                # tower-lsp language server
│  ├─ mwl-dap/                # debug adapter
│  ├─ mwl-convert/            # PHP→MWL transpiler, .phpt→.mwlt, migration report
│  ├─ mwl-pkg/                # package manager
│  └─ mwl-cli/                # the `mwl` binary
├─ tests/                     # cross-crate integration + conformance
├─ benches/                   # criterion micro + macro benchmarks vs PHP 8.5
└─ fuzz/                      # cargo-fuzz targets
```

### Unsafe policy

`#![forbid(unsafe_code)]` in every crate except three allow-listed modules in `mwl-runtime` and
`mwl-codegen`: the coroutine stack switcher, the request arena, and JIT page mapping. Those carry
`#![deny(unsafe_op_in_unsafe_fn)]`, a safety-invariant doc comment per block, dedicated Miri/ASAN
coverage, and require an ADR to grow. Prefer `corosensei` (audited, handles Windows SEH and aarch64) over
a hand-rolled switcher.

---

## Milestones

Each milestone ends with something runnable and its own tests. Do not start the next until the current
one's verification passes.

### M0 — Project setup (~3 days)
Install `rustup` (absent on this machine). Scaffold the workspace, all crate skeletons, CI matrix across
the three platforms, `clippy -D warnings`, `rustfmt`, `cargo-deny`, `cargo-fuzz`, dual licence files,
`CONTRIBUTING.md`, first commit on the empty repo, and the first ADRs recording the decision table above.

**Verify:** `cargo test` / `cargo clippy` / `cargo deny check` green on all three platforms in CI.

### M1 — Front end (~3 weeks)
Lexer with dual mode (`<?mwl`, `<?php`, `<?=`), inline HTML, heredoc/nowdoc, string interpolation, all
PHP 8.5 tokens. Recursive-descent parser covering the pragmatic-superset grammar: functions, classes,
interfaces, traits, enums, attributes, `match`, closures and arrow functions, generators, named arguments,
spread, nullsafe, `readonly`, promoted constructor parameters, first-class callable syntax, property hooks,
asymmetric visibility. Rejects `eval`/`$$var`/`goto`/`global`/`extract` with a diagnostic naming the
replacement. Error recovery good enough for the LSP.

**Verify:** `mwl ast file.mwl` dumps the AST; `insta` snapshot tests; `cargo fuzz` on the lexer and parser
finds no panic in a 1h run; parse the full local PHP 8.5 install's `.php` files without crashing.

### M2 — HIR, types, IR (~4 weeks)
Name resolution, namespaces and `use`, class hierarchy with trait flattening, statically resolved
`require`/`include` with a dynamic fallback. Gradual type system: declared types checked, locals inferred,
`mwl check` reports provable mismatches. Lowering to a CFG/SSA IR carrying explicit safepoints, refcount
operations and runtime-helper calls.

**Verify:** `mwl check` on a curated corpus of correct/incorrect programs; IR snapshot tests; typed
programs produce no `Unknown` types in the IR dump.

### M3 — Baseline Cranelift backend → **Hello World** (~3 weeks)
Calling convention (`fn(*mut Frame, *mut Ctx) -> ValueOrUnwind`), runtime helper table, `echo`, string
concat, arithmetic, comparison, control flow, function calls, safepoint polls, W^X page management, unwind
registration (SEH on Windows, DWARF elsewhere).

**Verify:** `mwl run hello.mwl` prints `Hello World` from natively compiled code on all three platforms.
Backtraces resolve through JIT frames. `mwl run --dump-asm` shows generated code.

### M4 — Language completeness — a usable CLI language (~10 weeks)
Full ordered-hash arrays with COW, exceptions unwinding correctly through JIT frames, closures with bound
`$this`, inheritance/interfaces/traits/enums, generators (nearly free given stackful coroutines), `foreach`
and iterators, references (`&$x`), static and instance members, magic methods, core string/array/math
functions, `var_dump`/`print_r`/`json_encode`.

Also in this milestone: `mwl test` and the `.mwlt` format — deliberately defined as a **superset of
`.phpt` sections** (`--TEST--`, `--FILE--`, `--EXPECT--`, `--EXPECTF--`, `--SKIPIF--`, `--INI--`,
`--ARGS--`, `--ENV--`, `--CLEAN--`) so the M10 importer is mechanical rather than a rewrite.

**Verify:** hand-written conformance suite ≥ 1000 `.mwlt` cases green; a non-trivial CLI program (an
argument-parsing file-processing tool) runs correctly; no leaks under Valgrind/ASAN.

### M5 — Concurrency (~4 weeks)
Per-core runtimes, coroutine scheduler, `spawn` / `await` / `all` / `race` / `timeout`, `Channel` with
backpressure, `parallel_map`, cross-core worker dispatch with deep-copy-or-move, structured concurrency
(a task tree dies with its parent — no orphans), async-native file I/O, sockets, timers and HTTP client.

**Verify:** stress tests with 100k concurrent tasks; a deliberate deadlock test proves cancellation
works; `parallel_map` shows near-linear speedup across cores on a CPU-bound benchmark; ThreadSanitizer
clean.

### M6 — Config, limits, capabilities, disk cache (~3 weeks)
Directive registry with changeability classes, boot config parsing, per-request overlay, `ini_set`
semantics, capability enforcement at every syscall-touching stdlib entry point, safepoint-driven limit
enforcement, content-addressed artifact cache with integrity verification and a refusal to use a
world-writable cache directory.

**Verify:** adversarial suite — a script attempting to widen a boot-locked capability fails; memory/CPU
caps terminate runaway scripts with a catchable error; warm-cache CLI startup under 10 ms; a tampered
cache artifact is rejected.

### M7 — Built-in HTTP server (~4 weeks)
`mwl serve`: hyper h1 + h2c, per-core accept and dispatch, request → isolated task, superglobals
(`$_GET`, `$_POST`, `$_SERVER`, `$_COOKIE`, `$_FILES`), multipart and urlencoded body parsing with limits,
streaming responses, static-file serving, graceful shutdown and zero-downtime reload, structured request
logging, optional TLS via `rustls`.

**Verify:** the core requirement demonstrated under load — 10k concurrent cold requests for the same file
compile it **exactly once** (assert via a compile counter) with no stalled requests; a state-bleed test
suite proves nothing leaks between requests; path traversal, header injection and request-smuggling suites
pass; `wrk`/`oha` throughput compared against PHP 8.5 + FPM + opcache and recorded in `benches/`.

### M8 — Stdlib and databases (~16 weeks)
Two-tier regex with the `preg_*` layer; JSON; hashing and crypto (RustCrypto: sha2, blake3, argon2,
bcrypt, aes-gcm); date/time with PHP-compatible formatting; filesystem and stream abstractions; process
execution behind the capability gate; sessions; a PDO-like DB API with pure-Rust MySQL/MariaDB, PostgreSQL
and MS SQL Server drivers plus SQLite (documenting `rusqlite`'s C dependency as an explicit, audited
exception to the pure-Rust rule).

**Also in this milestone: author the `mwl:ext@1.0.0` WIT world.** It must be designed from the same
value-access model as the built-in functions, so the Tier 0 internal interface and the Tier 1 guest
interface are one design rather than two that drift. Writing it later would mean retrofitting.

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

### M11 — PHP transpiler (~8 weeks)
`mwl convert`: PHP source → AST → rewrite passes → idiomatic `.mwl` output. Mechanical rewrites where
possible (`global` → parameter passing, `extract()` → explicit assignment, simple `$$var` → match on a
map); annotated `TODO` diagnostics where not (`eval`, dynamic includes, unsupported `preg` constructs,
C extensions). `--check` mode emits a migration report without writing files. A `.phpt → .mwlt` converter
reuses the same pipeline to import PHP's test corpus as native MWL tests. A PHP project depending on a C
extension is reported as needing either a Tier 1 `.mwlx` replacement or a Tier 2 native one — the converter
cannot synthesise either, and says so rather than emitting code that fails at runtime.

**Verify:** convert a real open-source PHP project end to end and run its test suite under MWL; imported
`.phpt` cases run in `mwl test` with a tracked pass rate and failures triaged as bug vs intentional
divergence.

### M12 — Optimising JIT tier (ongoing)
Profiling counters, inline caches for property and method access (monomorphic → polymorphic →
megamorphic), unboxed int/float fast paths, inlining, refcount elision, escape analysis, deopt and OSR at
existing safepoints.

**Verify:** macro benchmarks show a multiple over the baseline tier and over PHP 8.5 with JIT; no
correctness regressions in the full conformance suite when the optimising tier is forced on.

### M13 — Optional FastCGI transport
Only if a deployment target requires it (shared hosting, IIS, an existing nginx estate). Implements the
same `Transport` trait as `mwl-http`, with a fuzzed record parser and `SCRIPT_FILENAME` handling that
resolves within a configured root — closing the historical vulnerability class by construction.

---

## Immediate next steps

1. ~~Confirm `cranelift-jit` builds and executes a trivial function on Windows x86_64~~ — done; the spikes
   are now permanent guard tests in [`benches/abi-probe`](../benches/abi-probe/).
2. ~~Scaffold the workspace and CI (M0)~~ — done.
3. Write `docs/spec/00-overview.md`, and the remaining ADRs for the decision table, so the semantics are
   written down before code encodes them by accident. ADRs [0002](adr/0002-error-propagation.md) and
   [0003](adr/0003-extension-system.md) are written; the rest are not.
4. Begin M1 with the lexer, since inline-HTML mode plus interpolation shapes every layer above it.

## Overall verification strategy

- **Unit** — `cargo test` per crate; `insta` snapshots for AST/IR/codegen.
- **Conformance** — hand-written `.mwlt` suite as the normative definition of MWL; imported `.phpt`
  corpus tracked as a compatibility percentage.
- **Property/fuzz** — `proptest` for the array and string implementations; `cargo-fuzz` on lexer, parser,
  HTTP parser, multipart, regex and JSON, run continuously in CI.
- **Sanitisers** — Miri on the safe subset, ASAN/TSAN on the unsafe core and the scheduler.
- **Security** — the adversarial suites from M6/M7 (capability escape, resource exhaustion, cross-request
  state bleed, traversal, smuggling) plus `cargo deny` advisories on every build.
- **Performance** — `criterion` microbenchmarks and application-level macro benchmarks, always compared
  against the locally installed PHP 8.5.8, with results committed so regressions are visible in diffs.
