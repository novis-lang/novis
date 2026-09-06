`rule:errors/propagation`'s `nvs_helper!` wrapper contains a panic raised beneath a JIT frame. Code that runs on a worker with no request beneath it — the accept loop, the HTTP reader, the compiled-unit cache index — is outside it. So **the worker task's root is wrapped in `catch_unwind` as well**, applied by the task-spawning helper rather than written per call site, for the same reason `nvs_helper!` exists: a shape carries the obligation, so a future task cannot omit it by not knowing about it.

Two outcomes, split on whether a request owns the fault:

- **Beneath a request root** — the request fails through the escalation ladder as an internal panic, which `rule:errors/panics-bypass-user-code` routes past user code. The worker continues; its other in-flight requests are untouched, because everything the failing request owned is in an arena released wholesale (`rule:security/arena-is-an-ownership-root`).
- **Outside any request** — there is nothing to charge it to and the state that faulted is shared, so the worker is **retired**: it stops accepting, its in-flight requests finish under the existing drain, and a replacement is started. After an internal panic the runtime's own state is what cannot be trusted, and that reading applies to shared state as it does to request state.

This costs nothing on the path that does not panic, and is one wrap per *task* rather than per call.
