`[debug] mode` in `nvs.toml` states the default **and** the ceiling in one value: `[]` is off, and
any subset of `coverage`, `branch`, `trace` and `profile` is a ceiling a request may narrow and can
never widen. A production tree sets `mode = []` and no request-side call can turn any bit on.
Any other name stops the start where it is written.

`nvs run` and `nvs test` arm the probes the file names. **A server whose `[debug] mode` names any
probe fails its boot and its reload**, with an error that says why, the same on every platform. A
`trace` probe files two owned strings per compiled call with no ceiling, and nothing reads them on a
served request (`rule:observability/four-kinds-become-a-span`); `coverage` and `branch` count into a
table a served request does not have. Coverage of a live server is its own feature, not designed.

Whether a running request's internals are observable is not a request-local decision. An
attacker-controlled request that could turn tracing on for itself in production would gain a
reconnaissance channel over call arguments and timing, which is not a trade this language makes.

Writing a trace or a profile is its own capability — `debug.trace` and `debug.profile`, each granted
to named roots, deny-by-default, and **separate from the grant to write a file**. Being able to
write an ordinary file is not permission to persist a continuous log of every call's arguments,
which can carry request data a coverage-only deployment never needed to expose. A grant of
`debug.trace` is still a channel from request data to disk, so an operator names its roots
explicitly, the same way a path-rooted spawn grant is named. A query span rendered because `trace`
is on needs `debug.trace`; a span rendered because the request is sampled needs no grant.

**Declined:**
- **`[debug] mode` as a directive a request may set freely.** Observability of a request's own
  internals is not a request-local decision, for the reason above.
- **Arming the probes on a server with a cap on its `call` events.** The cap would be a new limit.
- **Refusing only `trace` and `profile` on a server.** `coverage` and `branch` would be accepted and
  count nothing.
- **Gating a sampled request's query span on the `debug.trace` grant.** Sampling is the request to
  export (`rule:observability/a-query-is-a-trace-event`); the grant gates only a span `trace` renders.
