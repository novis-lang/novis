Every compiled function and every runtime helper carries one signature, and errors travel in its
return value:

```rust
extern "C" fn(ctx: *mut Ctx, args: *const Value, out: *mut Value) -> i32
```

`0` is `OK` and the result is in `*out`. `1` is `THROWN`: an exception is pending in `Ctx` and a
`catch` may take it. `2` is `FATAL`: unrecoverable, bound for the request boundary, and not
catchable by Novis code — see `rule:errors/throwable-hierarchy` for why the type checker, rather
than runtime discipline, is what makes that true.

Codegen checks the status after every call and branches to an error block, which is where the
frame's refcount decrements and its `finally` blocks live — the explicit equivalent of a landing
pad. Nothing unwinds, because `cranelift-jit` registers no unwind tables with the OS on any
platform, so an unwinder would walk off a coroutine stack into unrelated memory. Error paths are
therefore ordinary IR the optimiser can see through, and a throw across a coroutine boundary is
not a special case.

A missing status check would swallow an exception without a trace, which is a worse failure than a
crash. So every call site `nvs-codegen` emits goes through one path, `emit_invoke_at`: a direct and
an indirect call differ only in where the callee comes from, and the argument slots, the status
check and the landing block are the same code for both.

Checked returns stay even if `cranelift-jit` learns to unwind through JIT frames, because coroutine
stack switches and error-path refcount drops the optimiser can see still favour them. The canary
`benches/abi-probe/tests/unwind_unavailable.rs` fails when that happens. It compiles a JIT function
with `unwind_info` on and asks the platform unwinder for an entry covering a PC inside it:
`RtlLookupFunctionEntry` on Windows, `_Unwind_Find_FDE` on Unix. Both see a table registered at run
time as well as the loaded images, and neither answer depends on the stack layout a provoked panic
would walk. A control lookup on a function of the test binary keeps a lookup that never finds
anything from passing. When the canary fails, the sentence about unwind tables above is rewritten
and the canary turned around, so it records that the entry exists and fails if that changes back.

**Declined:**
- **Native unwinding through JIT frames.** It needs unwind tables, frame descriptions and
  personality routines maintained for every platform, and an unwinder crossing a coroutine stack
  switch can walk into unrelated memory. The refcount drops on a throw path would also sit in
  landing pads the optimiser treats as opaque.
- **Calling foreign code that unwinds directly.** Nothing may unwind through a JIT frame, so a
  library that raises its own exceptions is reached only through a Rust wrapper that stops them
  (`rule:errors/helper-abi`).
