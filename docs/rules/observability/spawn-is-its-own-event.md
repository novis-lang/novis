Each spawn construct — `spawn`, `spawn worker`, `spawn script`, and `Core\Process::spawn`, whose child
is the operating system's rather than an isolate — emits a `kind: spawn` event from its own runtime
routine, and its join or result point closes it, gated by the same `TRACE`/`PROFILE` bits. The event
records the start timestamp, which form it was, the join timestamp, and a computed overhead split: the
parent-observed wall time minus the child-reported wall time that arrives with the child's answer. That
gives "real child compute" and "isolate scheduling and copy-out cost" as two numbers instead of one
opaque total.

A child process reports no wall time of its own, so its event carries the parent-observed wall alone
and no split, and its result point is the handle's `wait` (`rule:core-classes/process-spawn`) — a
handle the program stops reading from is killed with its task and leaves the event unjoined, which is
what a cancelled child reads as either way. The histogram `rule:observability/default-series` names is
the three isolate kinds and is not widened by this: a child process's cost is the child's, and the
series answers what spawning an isolate costs this runtime.

The form is what the source line *did*, not what it was written as: the same `spawn script … on:
"worker"` is a `spawn worker` when it starts the child on another core
(`rule:concurrency/on-worker-runs-the-child-on-another-core`) and a `spawn script` when it stays here,
because a reading that spells both the same way can say what a child cost but not what the core cost.

A spawn has its own instrumentation point because it is not a function call and does not flow
through the call probe — `rule:security/isolate-shares-nothing` rejects modelling it as one.

The child's own trace or profile stream is **never merged live** into the parent's during the spawn;
that would cross the arena boundary isolation exists to keep, and what crosses back is the child's
answer and its wall time, never its events. Showing a child's events nested under its parent's `spawn`
bar, anchored at the spawn's timestamp, is therefore an export-time reading and never a live one —
what carries a child's events to an export at all is `rule:testing/debug-probes`'s sink's business,
and nothing about it is a new cross-arena mechanism here.
