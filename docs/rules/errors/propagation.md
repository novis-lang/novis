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

Checked returns stay even if `cranelift-jit` learns to unwind through JIT frames, because coroutine
stack switches and error-path refcount drops the optimiser can see still favour them. The canary
`benches/abi-probe/tests/unwind_unavailable.rs` fails when that happens. The answer is then one
decision record that rewrites the sentence about unwind tables above and turns the canary around, so
it records that unwinding works and fails if that changes back.
