Guest execution spends the calling request's own budget. Its CPU time is the request's: at every
epoch tick the guest checks the request's CPU deadline and traps past it, so a deliberately infinite
guest loop stops rather than hanging a core (`rule:packaging/a-guest-call-yields-on-its-core`). Its
linear memory is the request's: each growth is charged to the request's memory accounting, and the
store's limit is the least of what the request has left, the entry's optional `memory` ceiling and
the manifest's declared maximum. What crosses from the guest is the request's memory as well: the
host's copy of a value the guest passes or returns is bounded by what the request has left and charged
to it while the host holds it, so a result is as large as the request can afford and no larger. An
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
ends — O(in-flight). The pooling allocator reserves address space per slot, not committed memory.
A result crosses as one host value per element before it is converted, tens of bytes for each byte
of a `list<u8>`, so a PDF of a few megabytes briefly holds over a hundred megabytes of the request's
memory. That is the
dynamic crossing's cost on the request path, paid once per call; a typed crossing for `bytes` is what
would lower it, at the price of a second conversion path.

`nvs_ext::call` charges every growth to a `Budget` and traps a guest past its CPU deadline or the
least of the three memory limits (`crates/nvs-ext/tests/call.rs`). The budget `nvs run` and
`nvs serve` hand a guest, `nvs_cli::extensions::Spent`, is the request's own: its CPU deadline is the
`CPU_LIMIT` flag the watchdog raises on the request tree, against everything the request has run since
it started, and each growth is charged to the memory count the runtime keeps for the request, under the
same `[limits] memory` ceiling as the program's own values. A limit's outcome is a `FATAL` naming the
limit as `onLimit` receives it (`crates/nvs-ext/tests/failure.rs`), and the request's limit handler
runs before it, in `nvs run`
(`tests/conformance/ext/an-extension-that-reaches-the-memory-limit-runs-the-limit-handler.nvst`) and
in `nvs serve`, which then serves the next request (`crates/nvs-cli/tests/serve_isolation.rs`).
