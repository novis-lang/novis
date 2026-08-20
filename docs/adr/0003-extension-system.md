# ADR 0003 — Extensions are sandboxed WebAssembly components, not native shared libraries

- **Status:** Accepted
- **Date:** 2026-08-20
- **Validated by:** [`benches/abi-probe`](../../benches/abi-probe/) — `tests/wasm_sandbox.rs` and the
  `wasm-probe` cost guards. Originally spike #4, on wasmtime 41 + cranelift 0.128,
  `x86_64-pc-windows-msvc`.
- **Amended by:** [0011](0011-functions-and-constants-are-class-members.md) — Tier 0's "fine-grained
  primitives" are `static` methods on `Core` domain classes, never bare functions; a manifest registers
  classes, whose `static` methods and `const` members the host adds to the symbol table, not functions or
  constants directly.

> **In short:** third-party extensions are sandboxed WebAssembly components (`.mwlx`), never
> shared libraries loaded with `dlopen`. Three tiers: built-in (`mwl-stdlib`), wasm component, and
> statically linked native. Values cross as bounds-checked handles rather than pointers, instances
> are fresh per request, and the guest gets no ambient authority. `dlopen` is rejected because it
> would destroy both memory safety and request isolation, which are the two claims the product
> rests on.

## Context

MWL needs to be extensible the way PHP is: a large built-in standard library, plus the ability to add
**precompiled** extensions without rebuilding the runtime. Three requirements, in tension:

1. **Easy to develop for.** PHP's extension API effectively requires C, manual refcount management, and
   intimate knowledge of the engine. That is why the extension ecosystem is small relative to the userland
   one.
2. **As secure and performant as built-in features.**
3. **Cross-platform precompiled artifacts**, where technically possible.

Requirements 2 and 3 are what decide this, and they point the same way.

## Options considered

**Native shared libraries (`.dll`/`.so`/`.dylib`) via `dlopen` — the PHP, Python and Node model.**
Native speed and direct heap access, and it is the obvious thing to copy. Rejected, decisively:

- It destroys memory safety, which is the central product claim. An extension runs in the process address
  space with no sandbox; a single out-of-bounds write corrupts arbitrary memory, including another
  request's.
- It destroys request isolation. MWL serves every request from one process, so an extension segfault
  takes down every in-flight request. In PHP's process-per-request model this is bad; here it is
  catastrophic.
- It bypasses the capability system entirely. Native code calls `open()` and `connect()` directly, so
  every grant in the root-owned `mwl.ini` becomes advisory rather than enforced.
- It cannot deliver requirement 3. Binaries need building per OS × architecture, and because Rust has no
  stable ABI, an extension would additionally have to match the host's compiler version and allocator or
  invoke undefined behaviour.
- The empirical case is PHP's own extension CVE history.

**Rust dylibs with a versioned stable ABI (`abi_stable`).** Better ergonomics for Rust authors, but
identical on every point above: no sandbox, no capability enforcement, per-platform binaries. Same
rejection.

**Out-of-process extensions over IPC.** Excellent isolation, but a round trip costs microseconds, which is
two to three orders of magnitude worse than the alternatives for anything fine-grained.

**Extensions written in MWL itself.** Perfectly safe and portable, and MWL will have this — it is what
`mwl pkg` distributes. But it cannot wrap an existing C or Rust library, which is the main reason
extensions exist. Complementary, not a substitute.

**WebAssembly components.** Chosen. See below.

## Decision

A three-tier system. The tiers are not a compromise; each exists because it is the right answer for a
different class of code.

### Tier 0 — built-in (`mwl-stdlib`)

Compiled into the binary. Native speed, direct heap access, no boundary at all. This is where
**fine-grained primitives** live: string and array operations, arithmetic and conversion helpers, anything
whose total cost is comparable to a function call — each a `static` method on a `Core` domain class rather
than a bare function ([ADR 0011](0011-functions-and-constants-are-class-members.md)).

### Tier 1 — WebAssembly component extensions (`.mwlx`)

**The default and recommended path for third-party extensions.** A `.mwlx` file is a WebAssembly component
carrying an `mwl.manifest` custom section — a single file, no archive.

- **One binary, every platform.** Requirement 3, fully satisfied rather than partially.
- **Memory-safe by construction.** A component cannot address host memory. Requirement 2's security half,
  satisfied more strongly than a built-in native function, which has no such guarantee.
- **Language-agnostic.** WIT plus `wit-bindgen` generates bindings for Rust, C, C++, Zig, Go, JavaScript
  and Python. Requirement 1, satisfied far better than PHP's C-only reality.
- **Governed by the same capability system** as everything else, because a component has no ambient
  authority.

### Tier 2 — statically linked native extensions

A Rust crate compiled into a custom `mwl` binary. For first-party subsystems that need raw sockets, TLS
termination, or direct heap access: the database drivers, the regex engine, crypto. Native speed, and safe
because it is safe Rust. Requires building from source, so it is an operator decision rather than something
downloaded — which is exactly the right friction for code that runs unsandboxed.

`mwl-db` and `mwl-regex` are Tier 2. That is not a workaround; it is where the standard library lives.

## Interface: WIT and the Component Model

MWL publishes a versioned world, `mwl:ext@1.0.0`. An extension implements it.

This buys three things beyond convenience. Rich types — records, variants, lists, strings, results,
resources — instead of marshalling everything through `i32`. Generated bindings in the author's language of
choice. And semantic versioning as part of the contract, which fixes the problem where a PHP extension must
be recompiled for every minor engine release.

### Values cross the boundary as handles, never pointers

MWL values stay in the host heap. The guest receives an opaque `value` resource — an index into a
per-call handle table that the host bounds-checks — and reads through host accessor functions.

- The guest cannot forge a host pointer. It can only present an index, which is validated.
- The host remains authoritative for refcounting and copy-on-write. The guest never sees a refcount.
- Large arrays and strings are not copied wholesale; the guest pulls only what it reads.
- For byte strings the guest may request a bulk copy into its own linear memory, measured at 11.7 ns per
  KiB — memcpy-bound, so effectively free.

### Extension functions are statically typed

At load time the host reads the manifest — declared classes, with their `static` methods and `const`
members, and any `mwl.ini` directives the extension wants — and registers them into the compiler's symbol
table. There is no separate function- or constant-shaped registration: an extension follows the same
class-only shape [ADR 0011](0011-functions-and-constants-are-class-members.md) requires of user code.
Consequently `mwl check` **type-checks calls into extensions at compile time**, and codegen emits a direct
call to the extension trampoline rather than a dynamic dispatch. PHP cannot do either.

## Isolation, limits and loading

**Loading is root-controlled.** `extension = image.mwlx` in the root-owned `mwl.ini`, consistent with
[the server-level configuration decision](README.md). A project cannot cause code to be loaded. Extensions
may be hash-pinned and signature-verified, since they are precompiled binaries arriving from outside.

**A fresh instance per request, created lazily.** Each request gets a pristine instance, so
**extension state cannot leak between requests** — a guarantee PHP does not offer, where a stateful
extension retains state for the lifetime of the worker. Instantiation is measured at 7.57 µs with the
pooling allocator, and is paid only for extensions a request actually calls. A typical request touches one
to three, so the realistic cost is 8–23 µs against a request budget measured in milliseconds.

**Compiled once, shared everywhere.** An extension is compiled on first load into the same
content-addressed artifact cache as MWL's own code, and the compiled module is shared across all cores via
`Arc`.

**Subject to the request's budget.** Epoch interruption ties guest execution to the per-request CPU cap;
spike #4 confirmed a deliberately infinite guest loop traps rather than hanging a core. Memory is capped via
`StoreLimits`. An extension therefore cannot starve its neighbours — again a stronger guarantee than a
built-in native function currently has.

**No ambient authority.** WASI is *not* granted by default. The guest receives only MWL's own
capability-checked host functions, so an extension's filesystem and network access is governed by the same
root-owned `mwl.ini` as script code. WASI is available as an opt-in world whose preopens are derived from
the capability grants.

**Async composes.** Wasmtime's async support is implemented with stack switching, which is the same
mechanism as MWL's stackful coroutines ([ADR 0002 corollary](0002-error-propagation.md)). An extension doing
I/O suspends the request's coroutine like any other MWL function — no async colouring, no special case.

## Why wasmtime rather than our own engine

MWL already embeds Cranelift, so writing our own wasm engine is tempting and would share most of the
backend. Rejected for the same reason MWL uses `hyper` instead of a hand-rolled protocol parser: the wasm
**validator** is security-critical, and a bug in it is a sandbox escape. Wasmtime is memory-safe Rust, is
the most-audited wasm runtime available, and is built on the same Cranelift version MWL already pins —
spike #4 confirmed wasmtime 41 and cranelift 0.128 coexist with no dependency conflict.

## Measured cost, stated honestly

From spike #4, release build:

| operation | cost |
|---|---|
| host → guest call | 11.5 ns |
| guest → host call (value accessor) | 9.0 ns |
| bulk copy 1 KiB into guest memory | 11.7 ns |
| fresh instance + one call, pooled | 7.57 µs |
| runaway guest stopped by epoch interruption | traps correctly |

For comparison, a built-in call frame costs 0.85 ns ([ADR 0002](0002-error-propagation.md)). **An
extension call therefore carries roughly 10 ns more overhead than a built-in call.**

The instantiation figure is the most environment-sensitive of these: 7.57 µs was measured in an isolated
binary, and the same code under a parallel test runner competing for cores measures ~17 µs. Both sit
inside the 8–23 µs realistic range quoted above, so the conclusion is unaffected — but quote it as a range
rather than a constant.

All five are guarded continuously in `benches/abi-probe/` (build the `wasm-probe` feature), which carries
criterion benchmarks for the costs and separate tests asserting the containment properties: that a guest
reading past the end of the host heap gets nothing rather than adjacent memory, that a runaway guest is
trapped by its deadline, and that a fresh instance cannot observe state written by a previous one.

That is noise for coarse-grained work — image codecs, compression, crypto, document parsing — and
significant for fine-grained work. This is precisely why Tier 0 exists for primitives and Tier 2 for
performance-critical first-party subsystems. The documentation must tell extension authors to design
coarse-grained APIs, because the boundary, not the compute, is what they control.

**Not yet measured:** in-guest compute throughput relative to native. Cranelift-compiled wasm is generally
within a small factor of native, but MWL should not ship a claim it has not measured. Benchmarking this
per-extension is a deliverable of the extension milestone, not an assumption.

## Consequences

**Positive**

- Requirements 2 and 3 are met without compromise, and requirement 1 is met better than PHP manages.
- A crashing or malicious extension harms one request, not the process. This is what makes the
  single-process server design defensible with third-party code in it.
- Extensions inherit per-request isolation, capability enforcement and resource limits for free, because
  they are enforced by the sandbox rather than by extension authors remembering to.
- Extension calls are statically type-checked.
- The ecosystem is open to any language with a wasm target rather than to C programmers only.

**Negative**

- ~10 ns per call versus a built-in. Accepted, and mitigated by tier placement rather than by pretending
  it is zero.
- Wasmtime is a substantial dependency. Accepted on the same grounds as `hyper`: security-critical parsers
  are not ours to write.
- Two host interfaces must be kept aligned — the internal one Tier 0 uses and the WIT world Tier 1 uses.
  Mitigation: author the WIT world *during* the stdlib milestone, from the same value-access design, so
  they are the same shape rather than two designs that drift.
- Extension authors who want direct heap access cannot have it. This is the point, not an oversight.

## Revisiting

Reopen if in-guest compute throughput measures far worse than expected for a real workload, in which case
the answer is to move that specific capability into Tier 0 or Tier 2 — not to open a `dlopen` path. Native
dynamic loading should be reconsidered only if MWL abandons either the memory-safety claim or the
single-process server model, at which point much else in this design changes too.
