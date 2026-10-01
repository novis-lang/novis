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

**Not on disk.** There is no guest call in the tree; `benches/abi-probe` proves epoch interruption on a
plain thread only.
