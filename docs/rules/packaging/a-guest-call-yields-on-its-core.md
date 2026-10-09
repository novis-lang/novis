A guest call runs on the calling request's own core, as a wasmtime async call that the request's
coroutine polls. A pending call parks the coroutine exactly as a socket wait does, and the core's
`RemoteWake` is its waker. wasmtime's fiber holds the wasm frames and saves its thread-local call chain
when it suspends, which is what lets a second coroutine on the same thread enter wasm safely while the
first is parked.

A call suspends in two cases. A host import that waits — a granted file read on the blocking pool, a
granted HTTP call through `Core\Http\Client` — returns a pending future. And an epoch ticker advances
the engine's epoch about once a millisecond: at each tick the guest checks the request's CPU deadline,
traps past it (`rule:packaging/a-guest-runs-under-the-requests-budget`), and otherwise yields so the
other tasks on that core run. A long encode slows its core's neighbours fairly and never freezes the
core, and no call pays a thread handoff.

A request has one instance per extension it calls (`rule:packaging/a-fresh-instance-per-request`). A
component cannot be re-entered, so a second task of the same request calling the same extension waits
until the first call returns.

**On disk.** `nvs_ext::call::Request::call` is the call, a future that parks on a pending import
and yields at every tick, and a second task of the request waits for the instance
(`crates/nvs-ext/tests/call.rs`). `nvs_host::block_on::block_on_yielding` drives it on the calling
task (`crates/nvs-cli/src/extensions.rs`). Its waker fires the core's `RemoteWake`. A tick's yield
fires that wake and then parks, so the task runs again behind the core's ready tasks at the reactor's
next poll, with no deadline armed; a pending import parks until its own wake
(`crates/nvs-host/src/block_on.rs`). `benches/abi-probe` proves the bridge with a component guest
under its `wasm-probe` feature: a second coroutine runs a whole guest call on the thread while the
first is parked inside wasm, and the CPU deadline traps a call that keeps yielding.
