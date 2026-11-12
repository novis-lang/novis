The safepoint poll bounds Novis code because it sits between calls. A helper whose runtime is O(its input) — a sort, a scan, an encode, a hash over a large value — is one call, and a deadline that can only fire between calls cannot fire inside it.

**A helper whose runtime scales with its input polls the request's deadline flag**, and the flag lives in `Ctx`'s hot cache line — the one `rule:errors/on-limit`'s stack check already loads, so the poll adds a compare against a line that is already resident and no load. The poll is amortised over a batch of iterations, and the batch is chosen so the amortised cost stays under the stack check's own per-call cost; that bound is what the perf guard holds. Reading a flag rather than a clock is the whole of why this is affordable: a clock read amortised over the same batch would cost more than the iteration it protects.

Two constraints on where a poll may go, both about correctness:

- **The poll is supplied by a bounded-loop combinator, not remembered per helper.** A helper adopts the shape and the shape carries the obligation. `nvs_runtime::bounded_loop` is the combinator for a loop the helper drives. `nvs_runtime::DeadlineBatch` is the batch counter for a loop another crate drives, such as a sort library's key function: it counts steps, reads the deadline once a batch, and stays stopped once it fires, so the site gives a constant answer and turns the stop into the same FATAL at the end.
- **Which walks poll.** A walk polls when its cost per byte is more than a copy: hashing, sorting, transcoding, parsing, matching. `Core\Hash` and `Core\Encoding` walk their input in pieces of 4 KiB through `bounded_loop`, and the native sort's key function polls through `DeadlineBatch`. A walk that only copies is bounded by the bytes the memory ceiling let it hold. Each walk that does more and does not poll yet has a gap record under `data/gaps/`.
- **A poll site must be a point at which abandoning leaves the value consistent.** A sort cannot be abandoned mid-permutation and its array handed back. Where no such point exists, the bound belongs on the *input* instead — which is what `rule:core-classes/regex-two-tiers` already did for patterns, and the precedent generalises rather than being re-argued.

**A stream read that never blocks is bounded at the stream.** A peer that keeps bytes arriving —
endless notices to a database driver, endless fragments to a WebSocket codec — makes no read park, so
neither the park funnel nor the stream's deadline is ever read. `NvsStream`'s `read` and `write`
count the calls that returned bytes since the stream last parked. At every sixteenth the next call
first checks: a passed stream deadline, or the running tree stopped for `WALL_TIME` or `CPU_LIMIT` or
past its park bound, ends the call with `TimedOut`. A raised `YIELD` is lowered and the task gives
the core back. This covers every protocol loop over a stream at one place. What it spends: one byte
per stream, one add per read, and per sixteen reads that did not park one thread-local read, one load
and one clock read. A read that parks pays nothing. Declined: a clock read on every read, and a check
in each driver loop.

**The safepoint poll also yields the core.** A request that has run a slice without waiting finds `YIELD` raised at its next poll and gives its core back there (`rule:http-server/a-request-that-computes-shares-its-core`). A helper's batched poll reads the deadline alone and does not yield: a helper call is one turn, and it holds its core until it returns or its deadline stops it. What bounds that turn is this rule's deadline poll, and what bounds the neighbours' wait is that a helper's runtime is O(its input) under the request's own limits.
