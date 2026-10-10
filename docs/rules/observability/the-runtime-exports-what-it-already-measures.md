Production telemetry — the default metric series, the spans, the trace ids on a log line — is read
from instrumentation the runtime already has: the per-statement and per-call probes of
`rule:testing/debug-probes`, and the `query`, `gc` and `spawn` event kinds filed inside routines
that are already slow. **No probe site is added to the per-statement/per-call path** for export,
and that path's measured cost is a guard that a change to the export must leave untouched.

One instrumentation, four consumers. Coverage, the timeline, the deterministic profiler and the
production export all read the same events, so a number cannot disagree with itself depending on
which tool asked. The distance between "measured" and "exported" is where a runtime without this
goes wrong, and closing it inside the runtime is the whole of this chapter. The two numbers no
outside agent can give, GC pause and isolate spawn, are the ones that explain an unexplained p99.

What it exports is `rule:observability/default-series`; which events become a span is
`rule:observability/four-kinds-become-a-span`; what an application adds by hand goes through
`rule:observability/metrics-three-members`.

**Declined:**
- **Leaving export to an extension or to userland.** The numbers that matter most, GC, spawn and
  arena memory, are inside the runtime and unreachable from outside it, and the two outside options
  are an unsandboxed C agent in the request path or a client reimplemented per framework.
