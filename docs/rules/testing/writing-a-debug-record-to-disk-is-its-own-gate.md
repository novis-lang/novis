`[debug] store` decides whether debug records are persisted at all, is off by default, and can only
subtract — no run mode, no log level and no other directive turns it on.

It is deliberately not a reading of the run mode and deliberately not a level threshold. `[log] level`
is a legitimate operational dial whose production default is `Info` and which a tree may legally
raise to chase a bug; if the debug stream rode it, that ordinary action would silently begin
persisting complete typed dumps of request data. **A reasonable operational dial must not double as a
data-exposure switch.**

So this is a second gate read together with every ceiling already in force, and off wherever either
says off. `rule:testing/debug-mode-directive`'s reconnaissance argument does not weaken because a
second key said yes, and a tree that writes no `[debug]` block gets no debug stream at all. The key is
`RuntimeTighten`, as `[debug] mode` and `[debug] inline` are: a request may turn its own recording off,
and no code can turn it on. What it turns on is the spool
(`rule:testing/the-debug-stream-reaches-the-service-through-the-disk`); what a request's record holds
in `development` is `rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`'s, and
which process starts the service is
`rule:testing/nvs-debug-is-the-service-and-a-development-server-starts-it`'s.

With the gate on, every request records its bounded span events (`query`, `http`, `spawn`) whatever
the sampling decision, because the spool is where they go. The unbounded `call` events still need the
`trace` probe bit (`rule:testing/debug-probes`).

With the gate off nothing reaches the disk, so a debug record that survives into a deployment costs
the predicted-not-taken branch `rule:testing/debug-probes` already pays and nothing further.

**Not on disk.** `[debug] store` is read by `nvs_config::Request::debug_store`, which
`nvs_runtime::spool::record` and `Ctx::records_spans` ask, but no request writes a record through
`nvs_runtime::spool::record` yet, so nothing reaches the disk.
