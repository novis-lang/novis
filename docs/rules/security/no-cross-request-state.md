Nothing a request does is observable by another request except through an explicit, capability-gated
store. Two things that look unrelated are the same violation and are closed together.

**Shared memory and process-wide IPC** — a shared segment reintroduces exactly the channel isolation
exists to eliminate, and there is no safe-if-careful version, because the entire isolation argument is
that carefulness is not a mechanism. **Userland calls that mutate process-global configuration** —
the environment, the locale, a numeric scale, an internal encoding, a default timezone — are ambient
mutable state read by later, unrelated code, and several are outright unsound in a multithreaded
process. The environment is read-only after startup, and locale, scale and timezone are always
explicit arguments.

This does not close operator configuration, which is governed, per-request, and cannot widen an
operator's ceiling. The rule is about **userland calls whose effect outlives or escapes the caller's
own request**. The replacement is a per-core or real shared cache, where the sharing is explicit,
bounded, and visible in the grants.
