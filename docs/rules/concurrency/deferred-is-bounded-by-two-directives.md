```toml
[deferred]
max_concurrent = 256      # System   — request trees per core kept alive for deferred work
deadline       = "30s"    # Runtime  — the default a call inherits when it names none
```

**`max_concurrent` is `System`.** It bounds how much a core holds after responses are on the wire,
which is a host-sizing decision and not a request-local one, so a request cannot raise it. Note that
it counts **trees**, not registrations: one request with ten deferred callables is one.

**`deadline` is `Runtime`** — an ordinary per-request default a call may name its own value for,
bounded like every other limit by the tree's remaining budget.

What it spends, stated so an operator sizes it rather than discovers it: a deferred tree holds its
arena at its peak for the length of its deferred work, so the worst case per core is `max_concurrent
× [limits.hard] memory` **on top of** in-flight requests. That is bounded, and it is O(in-flight
deferred trees) rather than O(requests served).
