`[debug] mode` in `nvs.toml` states the default **and** the ceiling in one value: `[]` is off, and
any subset of `coverage`, `branch`, `trace` and `profile` is a ceiling a request may narrow and can
never widen. A production tree sets `mode = []` and no request-side call can turn any bit on.
Any other name stops the start where it is written.

`nvs run` and `nvs test` arm the probes the file names. **A server whose `[debug] mode` names
`branch`, `trace` or `profile` fails its boot and its reload**, with an error that says why, the
same on every platform. A `trace` probe files two owned strings per compiled call with no ceiling,
and nothing reads them on a served request (`rule:observability/four-kinds-become-a-span`); `branch`
counts edges nothing on a server reads.

**A development server may name `coverage`**, and only `coverage`: the tree's `[mode] default` is
`development`, or the boot fails as above. Its compiler builds a located coverage table beside each
unit, and each request whose mode is `development` counts into a zeroed copy of its own, so
`Core\Debug::coverage()` names what ran and the request's spool `coverage` line carries it to the
debug viewer's heatmap (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`). A
production request under the same tree counts nothing. Whether a server counts is fixed at its boot,
and a reload that would start or stop counting is refused with a message to restart. Spends one
`u64` per statement and edge and one file and line per statement per unit, once, and one `u64` per
statement and edge per development request in flight, freed with the request; nothing where the
tree names no `coverage`, which is every production host.

Whether a running request's internals are observable is not a request-local decision. An
attacker-controlled request that could turn tracing on for itself in production would gain a
reconnaissance channel over call arguments and timing, which is not a trade this language makes.

Writing a trace or a profile is its own capability — `debug.trace` and `debug.profile`, each granted
to named roots, deny-by-default, and **separate from the grant to write a file**. Being able to
write an ordinary file is not permission to persist a continuous log of every call's arguments,
which can carry request data a coverage-only deployment never needed to expose. A grant of
`debug.trace` is still a channel from request data to disk, so an operator names its roots
explicitly, the same way a path-rooted spawn grant is named. A query span rendered because `trace`
is on needs `debug.trace`; a span rendered because the request is sampled needs no grant. A run with
`trace` on and no `debug.trace` grant writes one warning per process to the diagnostic channel, at its
first query, naming the line to add, so a trace that comes out empty is never silent.

**Declined:**
- **`[debug] mode` as a directive a request may set freely.** Observability of a request's own
  internals is not a request-local decision, for the reason above.
- **Arming the probes on a server with a cap on its `call` events.** The cap would be a new limit.
- **Refusing only `trace` and `profile` on a server.** `branch` would be accepted and count nothing.
- **Counting coverage on a production server.** A table per request on every host buys a heatmap
  only a developer reads, and a production tree is the one that must arm nothing.
- **Building the coverage table on a unit's first counted request.** It would be made on first use
  on the request path; the compile that builds the unit builds it.
- **Gating a sampled request's query span on the `debug.trace` grant.** Sampling is the request to
  export (`rule:observability/a-query-is-a-trace-event`); the grant gates only a span `trace` renders.
