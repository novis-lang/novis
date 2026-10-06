Guest execution spends the calling request's own budget. Its CPU time is the request's: at every
epoch tick the guest checks the request's CPU deadline and traps past it, so a deliberately infinite
guest loop stops rather than hanging a core (`rule:packaging/a-guest-call-yields-on-its-core`). Its
linear memory is the request's: each growth is charged to the request's memory accounting, and the
store's limit is the least of what the request has left, the entry's optional `memory` ceiling and
the manifest's declared maximum. An extension therefore cannot starve its neighbours — a stronger
guarantee than a built-in native function currently has, because it is enforced by the sandbox rather
than by discipline.

A guest that reaches the request's CPU or memory limit ends the request with a resource-limit `FATAL`,
which reaches `rule:errors/on-limit` like any other limit and never reaches the process. A trap that is
not a limit throws (`rule:packaging/a-guest-crash-throws`). Together with
`rule:packaging/a-fresh-instance-per-request`, this is what makes a single-process server defensible
with third-party code inside it: a crashing or malicious extension harms one request, not every request
in flight.

What it spends: the guest's linear memory, charged to the calling request and freed when the request
ends — O(in-flight). The pooling allocator reserves address space per slot, not committed memory.

**Not on disk.** `nvs_ext::call` charges every growth to a `Budget` and traps a guest past its CPU
deadline or the least of the three memory limits (`crates/nvs-ext/tests/call.rs`), but nothing
implements that budget over a request's own deadline and memory accounting yet. A limit's outcome
is a `FATAL` naming the limit as `onLimit` receives it (`crates/nvs-ext/tests/failure.rs`), and no
host raises it on a request yet.
