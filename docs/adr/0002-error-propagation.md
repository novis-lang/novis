# ADR 0002 — Exceptions propagate by checked return, not by unwinding

- **Status:** Accepted
- **Date:** 2026-08-20
- **Supersedes:** the "exceptions unwind through JIT frames" assumption in the original M3/M4 plan
- **Validated by:** [`benches/abi-probe`](../../benches/abi-probe/) — `tests/unwind_unavailable.rs`
  (the premise), `tests/invariants.rs` (propagation and containment), `tests/perf_guards.rs` (the
  cost). Originally spikes #1 and #2, on Rust 1.97.1 + Cranelift 0.128.4, `x86_64-pc-windows-msvc`.

> **In short:** exceptions and runtime errors propagate as a checked `i32` status returned from every
> call, never by unwinding — because `cranelift-jit` registers no unwind tables with the OS on
> any platform. Every runtime helper is `extern "C"` wrapping `catch_unwind`, which is what contains a
> runtime panic to one request. The normative signature is in **Decision**; the measured cost is in
> **Measured cost** and is guarded by the tests named above.

## Context

MWL compiles every function to native code with Cranelift and has no interpreter tier. Two things must
therefore cross native frames reliably:

1. **MWL exceptions** (`throw` / `catch`), which frameworks use heavily and sometimes for control flow.
2. **Runtime panics** — a bug in the Rust runtime, or a request hitting its memory limit — which must kill
   exactly one request and never the process, since one process serves every request.

The obvious design is to reuse the platform unwinder: emit unwind tables for JIT frames, let a Rust panic
or a custom exception unwind through them, catch it at the request boundary. That is what the original
plan assumed.

## Investigation

Spike #1 built a JIT function that calls a Rust helper, and made the helper panic. The helper was declared
`extern "C-unwind"` so the panic was permitted to escape it, and Cranelift was configured with
`unwind_info = "true"`.

Result: the process died with exit code `0xE06D7363` — the MSVC C++/SEH exception code — escaping
uncaught. `catch_unwind` sitting directly above the JIT frame never observed it.

The cause is in the crate source, not in our setup. `cranelift-jit` 0.128.4 contains no call to
`RtlAddFunctionTable` (Windows) or `__register_frame` (ELF/DWARF). Its only unwind-related code sits behind
the `wasmtime-unwinder` cargo feature, which implements **Wasmtime's own** exception mechanism — a private
side table walked by Wasmtime's custom unwinder — and is unrelated to the platform unwinder that Rust
panics use. `unwind_info = "true"` makes Cranelift *emit* unwind data; nothing ever registers it with
the OS.

This is not Windows-specific. The same absence applies to DWARF FDE registration on Linux and macOS.

Making native unwinding work would mean writing and maintaining, ourselves: `RUNTIME_FUNCTION` table
construction and `RtlAddFunctionTable` registration for Win64; `.eh_frame` FDE synthesis and
`__register_frame` registration for ELF; the `compact_unwind`/`libunwind` equivalent for Mach-O; a
personality routine per platform; and correct interaction between all of that and coroutine stack
switching, where the unwinder would walk off the end of a coroutine stack into unrelated memory.

## Decision

**MWL does not unwind. Errors propagate as an explicit status value checked after every call.**

The normative ABI for every compiled MWL function *and* every runtime helper:

```rust
extern "C" fn(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32
```

| status | meaning |
|---|---|
| `0` (`OK`) | success; the result has been written to `*out` |
| `1` (`THROWN`) | an MWL exception is pending in `Ctx`; a `catch` may handle it |
| `2` (`FATAL`) | unrecoverable (limit exceeded, internal error); unwinds to the request boundary and cannot be caught by MWL code |

Codegen emits, after every call:

```
    status = call callee(ctx, args, tmp)
    failed = icmp_imm ne status, 0
    brif failed, error_block, ok_block

ok_block:                        ; copy result out, continue
error_block:                     ; drop this frame's locals, return status onward
    return status
```

`error_block` is where this frame's refcount decrements and `finally` blocks go — the explicit equivalent
of a landing pad.

## Measured cost

Release build, `opt_level = "speed"`, on `x86_64-pc-windows-msvc`. The figure that matters is the
**marginal** cost of one more frame, measured as the slope between a 2-frame and an 18-frame chain so that
the benchmark's own call-out overhead cancels:

```
0.85 ns per frame        (6.2 ns at depth 2, 19.8 ns at depth 18)
```

The initial spike reported 1.3 ns by dividing a 3-frame total, which included that harness overhead; the
slope is the more accurate number and the one to hold the design to. For comparison, a single L2 cache
miss is roughly 10 ns. The branch is perfectly predicted on the happy path, so the real cost is the
instruction slot, not the branch.

**A throw costs slightly *less* than a normal return** — measured at 0.79–0.82× at depth 8 — because the
error path returns the status immediately while the success path also copies a 16-byte value up through
every frame. This is the claim that matters most for PHP compatibility, since frameworks throw on ordinary
control-flow paths.

That result only holds if throwing does not allocate. The probe originally stored its message as a
`String`, and that single allocation made a throw **2.8× a return** — more than the entire propagation
path it was meant to measure. The runtime must keep the same property: a static exception message must
not allocate, so the pending-error slot is a `Cow<'static, str>` rather than a `String`.

These numbers are guarded continuously rather than measured once. `benches/abi-probe/` carries both the
criterion benchmarks that track them and loose threshold tests that fail the build on an
order-of-magnitude regression.

## Consequences

**Positive**

- Zero platform-specific unwind code on 3 platforms × 2 architectures. This removes what was the single
  largest technical risk in the project.
- Composes with stackful coroutines by construction: there is nothing to unwind across a stack switch.
  Spike #3 confirmed throws propagate correctly through JIT frames living on a coroutine stack.
- Error paths are ordinary IR. The optimiser sees the refcount drops on the throw path and can move,
  merge or elide them. With native unwinding those live in landing pads the optimiser treats as opaque.
- `throw` costs the same as a return. PHP code that uses exceptions for control flow does not fall off a
  performance cliff.
- Debuggable: an error path is a branch you can step through, not a jump into the OS unwinder.

**Negative**

- One compare-and-branch per call on the happy path, which native unwinding would not pay. Measured at
  0.85 ns/frame; accepted.
- Every call site must be generated correctly. A missing status check silently swallows an exception, which
  is a nastier failure mode than a crash. Mitigation: call-site generation goes through a single
  `emit_call()` helper in `mwl-codegen` that always emits the check — no caller constructs a raw
  `call` instruction — plus an IR verifier pass asserting every call result is consumed by a branch.
- Foreign code that genuinely unwinds (a C library compiled with exceptions) cannot be called directly and
  must be wrapped on the Rust side. Acceptable: the stdlib is pure Rust by policy.

## Corollary: helper ABI and panic containment

Because nothing may unwind through a JIT frame, runtime helpers are declared `extern "C"` — **never**
`extern "C-unwind"` — and each wraps its body in `catch_unwind`, converting a panic into `FATAL` with the
message recorded in `Ctx`. `catch_unwind` costs nothing when no panic occurs. A single macro
(`mwl_helper!`) generates the wrapper so this cannot be forgotten per-helper.

This makes `panic = "unwind"` load-bearing rather than a preference: `panic = "abort"` would convert every
containable runtime bug into a process kill, destroying request isolation. It is set explicitly in every
profile in the workspace `Cargo.toml`.

A custom panic hook must be installed at startup so the message is routed to the request log with its
request id, rather than to the process's stderr.

## Revisiting

Reopen this decision only if all of the following become true: Cranelift ships platform unwind
registration as a supported API on all three target platforms; the optimising tier shows the status
branch to be a measurable bottleneck in application benchmarks (not microbenchmarks); and a design exists
for unwinding safely across coroutine stack boundaries. Until then, checked returns are not a workaround —
they are the better design.

The first of those conditions is checked automatically.
`benches/abi-probe/tests/unwind_unavailable.rs` re-executes itself as a child process, provokes a panic
beneath a JIT frame, and asserts the child is *terminated* rather than catching it. If a future Cranelift
starts registering unwind info, that test fails with a message pointing back here — so this ADR gets
revisited deliberately instead of quietly remaining true by inertia.
