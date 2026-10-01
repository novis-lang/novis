Third-party code is added to a running Novis as a **WebAssembly component** — a `.nvsx` file — and never
as a native shared library. There is no `dlopen` path, no `.dll`/`.so`/`.dylib` loader, and no FFI
(`rule:security/no-ffi`): a component cannot address host memory, so it is memory-safe by construction,
and it runs under the same request isolation, capability checks and resource limits as script code
because the sandbox enforces them rather than the extension author remembering to.

The engine is wasmtime, not a hand-rolled one. The wasm **validator** is security-critical — a bug in
it is a sandbox escape — and it is the same argument that puts `hyper` in front of a protocol parser
Novis did not write. Wasmtime is memory-safe Rust and pins the same Cranelift version the JIT already
uses, so the two coexist with one shared `cranelift-codegen`.

What this buys: one binary for every platform; bindings generated for any language with a wasm target
rather than C only; a crashing or malicious extension that harms one request and not the process; and
calls that are type-checked at compile time (`rule:packaging/extension-calls-are-statically-typed`).
A guest call is a wasmtime async call that the request's coroutine polls on its own core, so a guest
waiting on I/O or yielding at an epoch tick parks that coroutine like any other wait, with no async
colouring at the boundary (`rule:packaging/a-guest-call-yields-on-its-core`). What it costs is
`rule:packaging/the-boundary-is-the-cost`, and an author who wants direct heap access cannot have it.
That is the point.

**Not on disk.** wasmtime is used only by `benches/abi-probe`; nothing loads a component.
