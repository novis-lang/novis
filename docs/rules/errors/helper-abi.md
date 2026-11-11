Because nothing may unwind through a JIT frame, a runtime helper is declared `extern "C"` — never
`extern "C-unwind"` — and wraps its body in `catch_unwind`, turning a panic into `FATAL` with the
message recorded in `Ctx`. The `nvs_helper!` macro generates the wrapper, so it cannot be forgotten
one helper at a time, and `catch_unwind` costs nothing when no panic occurs.

`panic = "unwind"` is therefore load-bearing rather than a preference, and is set explicitly in
every workspace profile: `panic = "abort"` would turn every containable runtime bug into a process
kill, which is request isolation lost.

A **primitive** has no wrapper. It is a bare `extern "C" fn` that compiled code calls directly, such
as an array read or a refcount step, and it takes no `Ctx`, so it has no `FATAL` to report. A panic
in one aborts the process, and after it the runtime's state is unknown, so no program shape may reach
one. Each primitive, and each runtime function it calls, carries `#[deny(clippy::expect_used,
clippy::unwrap_used, clippy::panic, clippy::unreachable, clippy::indexing_slicing,
clippy::arithmetic_side_effects)]`. A site that stays is written `#[expect(<lint>, reason = "no
program reaches this: ...")]`. An operation that must report an error moves to a helper written
through `nvs_helper!`, with a landing block. `crates/nvs-runtime/tests/primitives_never_panic.rs`
reads the source and fails naming a primitive without the `deny` set. Declined: `catch_unwind` in
every primitive, and a status on every primitive's signature. Both cost a call on the hottest paths
for a fault the build can rule out.

A primitive that takes `Ctx` and returns a status may run its body under `abi::contain_panic` or
`run_helper`, which contain a panic to one request as the helper wrapper does. Its own body still
carries the `deny` set, and a function it reaches only through that wrapper is out of the sweep. A
primitive that returns nothing is never wrapped: it cannot report `FATAL`, and `Ctx::take_fatal` is
read only at a job seam, so a contained panic would surface later as an ordinary throw the program
can catch. Such a primitive, and each runtime function it calls directly, carries the set instead.

The wrapper does not stop at the helper. Code with no request beneath it — the accept loop, the
HTTP reader, the compiled-unit cache index — runs outside `nvs_helper!`, so a worker task's own
root carries a `catch_unwind` as well. And a panic raised while a panic is unwinding aborts the
process whatever the profile says, so nothing on a teardown path may panic and teardown does not
recurse.

A contained panic's message goes to the request log with its request id, not to the process's
stderr. `nvs_runtime::floor::install_panic_hook`, which `nvs` installs at startup, writes a panic
raised under a served request as a critical record and hands any other panic to the hook before it,
so a CLI script still prints its panic to stderr.
