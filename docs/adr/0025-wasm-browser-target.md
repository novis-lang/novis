# ADR 0025 — The browser is a second compile target, not a second language

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** a `wasm32` compile target for interactive client-side scripts running in a browser tab; its
  codegen backend and how it shares the CFG/SSA IR boundary from M2; the per-target capability matrix that
  excludes `spawn worker`/`spawn script`, coroutine-based suspension, and `.mwlx` wasm-component extensions
  for this target only; the new `Core` host-context domain for DOM/window interaction; `require`'s
  build-time-only resolution when there is no filesystem to fall back on.
- **Validated by:** none yet — the backend does not exist before M14. This ADR reuses the
  sandboxing/per-instance-cost properties `benches/abi-probe/tests/wasm_sandbox.rs` already established
  under [0003](0003-extension-system.md); its own target-specific claims (the dropped coroutine, the
  capability matrix) get their own guard test when M14 starts, per the *Revisiting* section below.
- **Relates to:** 0002, 0003, 0004, 0006, 0007, 0009, 0010, 0011, 0012, 0013, 0014, 0015, 0017,
  0020, 0022, 0023, 0024

> **In short:** MWL gains an optional `wasm32` compile target for running client-side in a browser tab,
> implemented as a **second codegen backend consuming the same IR** M2 already produces — not a second
> front end, type checker, or set of language semantics. Cranelift has no `wasm32` output (Wasmtime uses it
> the other direction), so this backend lowers IR to wasm opcodes directly. Three things are unavailable in
> this target specifically, each a compile-time diagnostic rather than a silent downgrade: `spawn worker`/
> `spawn script` ([0006](0006-isolated-script-execution.md), no matching concurrency substrate in a tab),
> coroutine-based suspension (the stack-switching mechanism behind it has no standard `wasm32` equivalent),
> and `.mwlx` extensions ([0003](0003-extension-system.md), no Component Model host in a browser). A fourth
> `Core` accessor domain covers DOM/window state, following [0012](0012-no-superglobals.md)'s existing
> pattern; its method table is explicitly *not* designed here. Every other language and stdlib feature —
> types, taint tracking, PHP-compat semantics, the checked-return ABI — is identical across every target.

## Context

- The other two deployment targets (server, CLI) differ only in *host context*, not language — a browser
  tab is a third context, further removed than those two are from each other: no inbound request, no
  process/argv, and no OS thread/process substrate to build `spawn` or a JIT's W^X pages on.
- This fits the existing model provided the target is scoped as producing a `.wasm` artifact ahead of time
  for the browser's own engine to instantiate — not running MWL's native JIT *inside* a wasm sandbox, which
  is impossible by construction (a wasm module can't mark its own memory executable or emit new callable
  code beyond wasm's own `instantiate`/`compile`, the same sandboxing property
  [0003](0003-extension-system.md) already relies on).
- Three already-committed runtime mechanisms don't survive that scoping unchanged, each addressed in
  *Decision* below: stackful coroutines (native stack-switching via `corosensei`, no `wasm32` equivalent
  short of Asyncify or an unshipped proposal); the isolate model
  ([0006](0006-isolated-script-execution.md), assumes an OS thread/process substrate); the extension host
  ([0003](0003-extension-system.md), Wasmtime running the Component Model, which no browser ships).

## Decision

### A second codegen backend, same IR

M2 lowers every compiled unit to one CFG/SSA IR with explicit safepoints, refcount operations, and runtime-helper
calls — built once, upstream of any backend. M3 lowers that IR to native machine code via Cranelift, in two
tiers (helper-call baseline, typed native instructions) that already share the same IR boundary. Cranelift
itself has no `wasm32` output; Wasmtime uses it to compile *from* wasm to native, the opposite direction. So
this target adds a third IR consumer: an instruction-selection pass from the same IR to wasm opcodes, via a
pure-Rust emitter (e.g. `wasm-encoder`, Bytecode Alliance, consistent with `deny.toml`'s pure-Rust-by-default
policy already trusted for Wasmtime under [0003](0003-extension-system.md)) — not a fork of the parser,
resolver, type checker, or taint checker, all of which run exactly once regardless of which backend
eventually consumes their output.

W^X page management and the optimising tier's runtime code-patching (deopt/OSR, slated for M12) simply don't
apply here: a `.wasm` module is emitted once, ahead of the browser loading it, and the browser's own engine
does whatever JIT/tiering it wants to the bytes MWL handed it. That is a simplification for this target, not
a quirk to work around.

### The capability matrix: what this target drops

Three features are native/webserver-only. Using one in a `wasm32`-browser compile is a diagnostic naming
this ADR, never a silent no-op or a weaker substitute:

1. **`spawn worker` / `spawn script`.** The isolate model spends an OS thread/process substrate: one
   immutable compiled-code `Arc` shared across cores, cooperating through a request-tree budget
   ([0006](0006-isolated-script-execution.md)). A browser tab's only concurrency substrate is Web Workers
   over `postMessage`/structured-clone, with no shared compiled-code cache and no shared memory without
   `SharedArrayBuffer` plus cross-origin isolation headers most embedding pages won't have. Rather than
   inventing a second, weaker isolation boundary for one target, `spawn` is unavailable when compiling for
   `wasm32`-browser.

2. **Coroutine-based suspension.** The mechanism is native stack-switching, which has no standard `wasm32`
   equivalent (see *Context*). Rather than special-casing suspension per target inside every `Core` function
   that might yield, the browser target runs every MWL function to completion synchronously: a `Core`
   function that would need to suspend on the native/webserver targets is not offered in this target's
   `Core` surface at all, for v1. The *available standard-library surface* differing by target, not the
   language, is exactly the shape [0012](0012-no-superglobals.md) already gives every target — ambient
   capability was always routed through `Core` accessor classes, never language syntax.

3. **`.mwlx` wasm-component extensions.** Tier 1 extensions load through Wasmtime as host
   ([0003](0003-extension-system.md)), which nothing in a browser tab provides — there is no shipped
   Component Model runtime to be that host. A browser build gets Tier 0 built-ins only; third-party
   extensions are out of scope for this target in v1.

Everything else is unchanged: the type system, PHP-compat semantics, taint tracking and its sinks, enums,
`Comparable`, property observers, name resolution, definite initialization, `clone`/`serialize`. None of
[0007](0007-explicit-type-system.md)/[0009](0009-string-and-bytes.md)/[0010](0010-enums-are-a-value-type.md)/
[0011](0011-functions-and-constants-are-class-members.md)/[0013](0013-comparable-interface.md)/
[0014](0014-property-observer.md)/[0015](0015-no-name-aliasing.md)/[0020](0020-error-escalation-ladder.md)/
[0022](0022-definite-property-initialization.md)/[0023](0023-clone-serialize-and-cross-boundary-copy.md)/
[0024](0024-taint-tracking-for-injection-sinks.md) gets a target-specific carve-out. A program touching none
of the three excluded features compiles and behaves identically whether it targets the server or a browser
tab.

### A fourth Core host-context domain

[0012](0012-no-superglobals.md) already routes every piece of ambient host state through a `Core` accessor
class instead of a superglobal: `Core\Request`, `Core\Server`, `Core\Session`, `Core\Env`, `Core\Cli`,
`Core\Script`. A browser tab is a context none of those six model — there is no inbound HTTP request, and
"the current script" is a loaded page, not a CLI invocation. This ADR reserves a **seventh accessor domain**
for that context (`Core\Browser`, working name) covering whatever DOM/window/event surface v1 exposes. Its
method table is explicitly deferred — that is stdlib-shaped design work for whenever M14 is scheduled, not a
decision made here; per this repo's "state a fact once" rule, it gets its own documentation when it exists.
What this ADR commits to is only the *shape*: a same-rules `Core` class the resolver treats like any other,
no new syntax, and — per [0012](0012-no-superglobals.md)'s existing rule for a spawned isolate — calling
`Core\Request`/`Core\Server`/`Core\Session` from a browser-target compile is refused at the same point,
since neither context exists there either.

### `require` becomes build-time-only

[0021](0021-single-file-inclusion-construct.md) makes `require` resolve statically where possible with a
dynamic fallback for a computed path, and throw on a missing file either way. The dynamic fallback assumes a
filesystem to resolve against at runtime; a browser tab has none. For the `wasm32`-browser target, every
`require` must resolve to a bundled module at compile time — a `require` whose path cannot be resolved
statically is a diagnostic in this target, where the same program would fall through to the dynamic path
server-side. `require`'s semantics (runs every time reached, throws on failure) do not change; only which
paths are legal narrows — the same shape a capability grant already gives a script without changing the
language underneath it.

## Consequences

**Positive**

- The language surface does not grow. No new keyword, no new type, no per-target dialect: a `.mwl` file
  that avoids the three excluded features is portable across all three targets unmodified.
- [0012](0012-no-superglobals.md)'s design pays for itself again — because host state was already behind
  accessor classes rather than ambient variables, adding a fourth context is additive (one new class), not a
  retrofit of existing call sites.
- The IR boundary M2 draws absorbs a third backend the way it already absorbs two tiers (helper-call
  baseline, typed native instructions), which is evidence that boundary sits in the right place.
- W^X page management, safepoint-driven CPU-limit enforcement, and OS-thread scheduling all simply don't
  apply to this target — less to build, not more.

**Negative**

- **A second full codegen backend is real, ongoing cost**: instruction selection and calling-convention
  lowering happen twice from here on for every codegen feature. Accepted under priority 4 (simplicity of the
  implementation, spent last, per AGENTS.md's priority ordering) because the target is optional — M14,
  contingent, the same status M13 already gives the FastCGI transport — and shares everything upstream of
  codegen.
- **The `Core` surface genuinely differs by target.** Code that spawns, suspends on I/O, or loads a `.mwlx`
  extension does not port to the browser silently; it is a compile-time diagnostic, not a runtime surprise,
  but "write once, target anywhere" now has three named exceptions instead of zero.
- **`Core\Browser`'s method table is undesigned.** This ADR commits to the shape — a seventh accessor
  domain, the same rules as the other six — and deliberately not to the contents.

## Alternatives rejected

- **Compile the native JIT itself to wasm and run it inside the browser, JIT-ing MWL source at page load.**
  Requires mmap'ing executable pages from inside a wasm sandbox, which no browser permits, and contradicts
  [0003](0003-extension-system.md)'s reasoning for why native code gets no ambient authority.
- **Emulate coroutines with Asyncify** to keep one execution model across every target. Instruments every
  function reachable from a suspend point, not only the ones that suspend — a whole-module cost
  [0004](0004-memory-for-simplicity.md) asks to be paid deliberately rather than by default.
- **Give the browser target its own weaker isolation primitive** instead of dropping `spawn` outright (e.g.
  silently degrading to a same-thread call). A construct whose isolation guarantee quietly disappears on one
  target is worse than one simply refused there.
- **Model the browser context as `Core\Request` or `Core\Cli` with browser-specific methods bolted on.**
  Invites the same "is this call legal in this context" ambiguity [0012](0012-no-superglobals.md) already
  closed by giving the CLI its own `Core\Cli`.

## Revisiting

Reopen the coroutine exclusion if a browser stack-switching mechanism (Asyncify or a native proposal) ships
broadly enough, and cheaply enough, that a wasm-side suspend/resume guard test in `benches/abi-probe` could
hold a cost bound worth shipping — that test not existing yet is itself the reason this stays excluded for
now, not a settled "never." Reopen the extension exclusion if the Component Model ships in browsers, or if a
non-component `wasm32` extension shape is ever defined for Tier 0/2 that a browser host could load directly.
Neither is assumed here — this ADR's target scope is the synchronous, extension-free subset, and M14 should
not silently grow past that without its own argument.
