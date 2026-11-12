Guest execution spends the calling request's own budget. Its CPU time is the request's: at every
epoch tick the guest checks the request's CPU deadline and traps past it, so a deliberately infinite
guest loop stops rather than hanging a core (`rule:packaging/a-guest-call-yields-on-its-core`). Its
linear memory is the request's: each growth is charged to the request's memory accounting, and the
store's limit is the least of what the request has left, the entry's optional `memory` ceiling and
the manifest's declared maximum. What crosses from the guest is the request's memory as well, and it
is counted once: the host's copy of a value the guest passes or returns is an allocation of the
request, which the request's allocator counts as it counts any other. Nothing measures it a second
time. A crossing is bounded by the hostcall fuel, which is what the request has left when the call
starts, and the call asks the request's memory after it lowers the arguments and again after it
lifts the results. A request over its ceiling at either point ends at the memory limit. So a 40 MiB
result is 40 MiB of the request's memory, not 80. Only guest linear memory, which is mapped and not
allocated, is charged by hand. A
host import that allocates a size the guest chose asks what the request has left before it allocates:
random bytes, `as-bytes` on a lent file or a byte string, `append` to a `bytes`, and a header
list, a header or a request body written through `wasi:http`. A size past it ends the call at the
memory limit, and nothing is allocated.

**The epoch tick asks the request's memory beside its CPU deadline.** Whatever put the request over
its ceiling — a logged line it holds until the call returns, a crossing value, an import that
allocated — the next tick ends the call at the memory limit. So every import, present and future, is
bounded without a charge of its own: between two ticks of 1 ms a guest makes the host allocate at
most what its imports allocate in that time, which for the log is the lines it writes in 1 ms. A
send or a read the guest asks for after the crossing is answered with a failure and does nothing,
and that answer wakes the guest, so a guest waiting on the host meets the same tick. Each
instance's resource table holds at most `nvs_ext::handle::MOST` streams, pollables and descriptors,
so a guest cannot grow it by asking in a loop. An
extension therefore cannot starve its neighbours — a stronger
guarantee than a built-in native function currently has, because it is enforced by the sandbox rather
than by discipline.

A guest that reaches the request's CPU or memory limit ends the request with a resource-limit `FATAL`,
which reaches `rule:errors/on-limit` like any other limit and never reaches the process. A trap that is
not a limit throws (`rule:packaging/a-guest-crash-throws`). Together with
`rule:packaging/a-fresh-instance-per-request`, this is what makes a single-process server defensible
with third-party code inside it: a crashing or malicious extension harms one request, not every request
in flight.

What it spends: the guest's linear memory, charged to the calling request and freed when the request
ends — O(in-flight). The pooling allocator reserves address space for each place's memory, and
commits its table pool at the engine's start, at most 1.25 GiB
(`rule:packaging/a-fresh-instance-per-request`).
The memory question at each tick is three thread-local reads per running guest, and nothing per
allocation. The one count saves a walk over every crossing value per call, and a crossing value's
bytes are counted once.
A result crosses as one host value per element before it is converted, tens of bytes for each byte
of a `list<u8>`, so a PDF of a few megabytes briefly holds over a hundred megabytes of the request's
memory. That is the
dynamic crossing's cost on the request path, paid once per call; a typed crossing for `bytes` is what
would lower it, at the price of a second conversion path.

`nvs_ext::call` charges every growth to a `Budget` and traps a guest past its CPU deadline or the
least of the three memory limits (`crates/nvs-ext/tests/call.rs`). `nvs_ext::call::afford` is the check
a host import asks first (`crates/nvs-ext/tests/wasi.rs`, `crates/nvs-ext/tests/convert.rs`). The budget `nvs run` and
`nvs serve` hand a guest, `nvs_cli::extensions::Spent`, is the request's own: its CPU deadline is the
`CPU_LIMIT` flag the watchdog raises on the request tree, against everything the request has run since
it started, and each growth is charged to the memory count the runtime keeps for the request, under the
same `[limits] memory` ceiling as the program's own values. Its `memory_spent` reads the counters the
allocator raises `MEMORY_LIMIT` from, on the request's own thread, so a collection that brought the
request back under is not a stop (`a_guest_is_interrupted_once_its_request_is_over_the_memory_ceiling`,
`a_guest_that_logs_without_end_is_stopped_at_the_memory_ceiling`,
`a_guest_cannot_fill_the_resource_table_past_its_cap`, `a_crossing_value_is_charged_to_its_request_once`, and the attack
`tests/hostile/config/limits/02-an-extension-that-writes-to-the-log-without-end.nvs`). A limit's outcome is a `FATAL` naming the
limit as `onLimit` receives it (`crates/nvs-ext/tests/failure.rs`), and the request's limit handler
runs before it, in `nvs run`
(`tests/conformance/ext/an-extension-that-reaches-the-memory-limit-runs-the-limit-handler.nvst`) and
in `nvs serve`, which then serves the next request (`crates/nvs-cli/tests/serve_isolation.rs`).
