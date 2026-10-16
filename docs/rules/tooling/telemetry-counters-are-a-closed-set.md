Any `nvs` invocation may increment counters in the local file — a write, never a connection. A counter is
`invocation.<subcommand>`, where `<subcommand>` comes from the binary's own compiled-in command table,
plus exactly one fallback, `invocation.unknown`. **The literal argv never becomes a counter name**: a
typo, a path or a misparse increments `invocation.unknown` and nothing else, so arbitrary user input
cannot leak into the schema, and every counter that can ever exist is enumerable from the binary that
wrote it. A closed set with one `unknown` bucket is the whole difference between a schema and a log.

Beside the counters the payload carries only a schema version, the `nvs` version, coarse `os` and
`arch`, and the ISO week being reported. No machine ID, no per-event timestamps, no arguments, no paths,
no environment.

**Nothing derived from traffic is ever counted.** `nvs serve` incrementing `invocation.serve` at startup
records the operator's action; request counts, routes, headers and anything else the internet sent belong
to the site's visitors, who never consented, and are permanently out of schema — not merely off by
default. The counter file is provably a function of what the operator did.

- **Memory:** a few KiB in one local file per user, and one buffered append per invocation. For
  `nvs serve` that append happens once, at startup, before the listener binds. Nothing is spent on the
  request path.

**Not on disk.** There is no counter file and no payload.
