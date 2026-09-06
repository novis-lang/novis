```
Log\Level::Debug   Log\Level::Info   Log\Level::Warn   Log\Level::Error   Log\Level::Critical
```

An ordinary enum, and the syslog mapping is fixed — 7, 6, 4, 3, 2 — because `syslog` is a
`rule:errors/engine-floor` target and a severity is not optional there.

`Critical` exists so `rule:errors/escalation-ladder` has a level of its own rather than a parallel
channel: a resource-limit fatal and a failed third-party call are not the same alerting decision,
and a level is where an alerting rule can read that difference. There is no `Trace` case, because
spans and sampling belong to tracing and a trace *level* would be a second home for that fact.

`[log] level` sets the minimum level written.
