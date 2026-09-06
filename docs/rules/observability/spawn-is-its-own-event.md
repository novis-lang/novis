Each of the three spawn constructs — `spawn`, `spawn worker`, `spawn script` — emits a `kind: spawn`
event from its own runtime routine, and its join or result point closes it, gated by the same
`TRACE`/`PROFILE` bits. The event records the start timestamp, which of the three forms it was, the
join timestamp, and a computed overhead split: the parent-observed wall time minus the child-reported
wall time that already arrives on `ScriptResult`. That gives "real child compute" and "isolate
scheduling and copy-out cost" as two numbers instead of one opaque total.

A spawn has its own instrumentation point because it is not a function call and does not flow
through the call probe — `rule:security/isolate-shares-nothing` rejects modelling it as one.

The child's own trace or profile stream is **never merged live** into the parent's during the spawn;
that would cross the arena boundary isolation exists to keep. Showing a child's events nested under
its parent's `spawn` bar, anchored at the spawn's timestamp, is an export-time operation in the CLI's
exporter over data that already crosses the boundary on `ScriptResult` — not a new live cross-arena
mechanism.
