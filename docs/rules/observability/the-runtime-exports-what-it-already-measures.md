Production telemetry — the default metric series, the spans, the trace ids on a log line — is read
from instrumentation the runtime already has: the per-statement and per-call probes of
`rule:testing/debug-probes`, and the `query`, `gc` and `spawn` event kinds filed inside routines
that are already slow. **No probe site is added to the per-statement/per-call path** for export,
and that path's measured cost is a guard that a change to the export must leave untouched.

One instrumentation, four consumers. Coverage, the timeline, the deterministic profiler and the
production export all read the same events, so a number cannot disagree with itself depending on
which tool asked. The distance between "measured" and "exported" is where a runtime without this goes wrong —
an unsandboxed C agent in the request path, or a userland client reimplemented per framework, and
neither able to see a GC pause or an isolate spawn — and closing it inside the runtime is the whole
of this chapter.

What it exports is `rule:observability/default-series`; which events become a span is
`rule:observability/four-kinds-become-a-span`; what an application adds by hand goes through
`rule:observability/metrics-three-members`.
