When a design trades memory footprint against security, latency or simplicity, Novis pays the memory.
The ordering, highest first; a lower item is spent to buy a higher one, never the reverse:

1. **Security and request isolation** — not traded for anything, including for the three below.
2. **Latency and throughput on the request path.**
3. **Simplicity** — of the language surface first, then of the implementation.
4. **Memory footprint** — last, and spent deliberately to buy any of the above.

**A priority is cited by its name, never by its number**, so a citation survives any change to the
list. What a program observably does is not in the ordering: the rules state it and the conformance
cases pin it.

The ordering follows what is scarce. Novis runs on server- and developer-class machines, where RAM
is cheap and sized in advance. One process serves every request, so a sandbox escape or a
cross-request leak is a bug in all of them; a request's budget is milliseconds, and every allocation
is spent out of it; and every invariant held by discipline rather than by construction is something
each later contributor must remember. RAM can be bought and a remembered invariant cannot, and the
places most likely to be wrong — the unsafe modules, codegen call sites, the sandbox boundary — are
where a memory-saving trick would have to live. One ordering stated once also puts the burden of
proof on the smaller option, whose cost is otherwise diffuse and lands in a different file.

**Novis is not a low-footprint runtime.** It does not target `no_std`, microcontrollers or minimum-RSS
deployments, and there is no cut-down build trading behaviour for size. "Uses less memory" is not on
its own an argument for a change; "uses less memory and is no more complicated" is simply a better
design and needs no appeal to any of this.

Three bounds, and none of them is licence to leak. **Bounded, not merely modest** — every per-request
allocation stays under a cap the runtime can *enforce*, and consumption that cannot be attributed to a
request and capped is not a trade-off but a hole in isolation. **O(in-flight), not O(served)** — cost
is proportional to concurrent work and released wholesale when that work ends; growth proportional to
requests served is a leak, and no ordering makes a leak acceptable. **Footprint, not traffic** — bytes
held are cheap and bytes moved are not, so an extra allocation or cache miss on a hot path is a
latency question, however much it looks like a memory one. The pending error's message is a
`Cow<'static, str>` rather than a `String` for that reason: one allocation per throw cost more than
the propagation path it was measured against (`rule:errors/propagation`), which reads as frugality
and is latency winning over footprint.

**Say what you spend.** A decision that takes memory states the amount, per request or per task, in
the doc comment or record that carries it. The ordering is only usable by a later reader if the costs
are written down as they are incurred; otherwise "a bit more memory" six times becomes a number
nobody chose. Nor is the ordering an excuse: "memory is cheap" never argues for an unbounded cache, a
leak or an uncapped buffer, and a review asks for the number.

Resident memory per process is therefore higher than a footprint-tuned runtime's, and it scales with
in-flight concurrency rather than request rate. An operator sizes for tasks times their stack plus
concurrent requests times their cap, not for one typical RSS, and the comparison Novis stands behind
is throughput and latency per core at a given concurrency with isolation intact. A deployment target
where memory is the billed or hard-limited unit — small per-tenant containers, fixed small serverless
budgets, embedded use — would move footprint up the ordering, and that is a change of direction, not
tuning.

**Memory:** what the language already spends under this ordering.
- **A stack reservation per in-flight task**, committed lazily, which is what keeps I/O free of
  `async` colouring (`rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`).
- **Sixteen bytes per value**, eight more than a NaN-boxed word, which is what keeps the full `int`
  range.
- **A refcount per array and string, and a full copy on the first write to a shared one**, which is
  value semantics with no aliasing rules to learn.
- **A copy of every value crossing an isolate boundary**, moved instead when its refcount is 1, which
  keeps refcounts non-atomic and data races impossible (`rule:security/isolate-values-cross-by-copy`).
- **Peak rather than average retention inside a request**, because the request heap is dropped whole;
  cycles cannot accumulate and no collector runs on the request path
  (`rule:security/arena-is-an-ownership-root`).
- **A fresh instance per extension a request calls**, in a store charged to that request, so extension
  state cannot leak between requests
  (`rule:packaging/an-extension-is-a-sandboxed-wasm-component`).

**Declined:**
- **A correctness item in the ordering**, with or without a clause naming PHP. Correctness is not
  something the ordering trades against latency or memory, so an item for it decides nothing, and the
  rules and the conformance cases already own it.
