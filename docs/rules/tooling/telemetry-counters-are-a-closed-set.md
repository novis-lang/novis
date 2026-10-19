Any `nvs` invocation may increment counters in the local file — a write, never a connection. A counter is
`invocation.<subcommand>` or `internal-error.<subcommand>`, where `<subcommand>` comes from the binary's
own compiled-in command table, plus exactly one fallback each, `invocation.unknown` and
`internal-error.unknown`. **The literal argv never becomes a counter name**: a typo, a path or a misparse
increments `invocation.unknown` and nothing else, so arbitrary user input cannot leak into the schema,
and every counter that can ever exist is enumerable from the binary that wrote it. A closed set with one
`unknown` bucket is the whole difference between a schema and a log.

`internal-error.<subcommand>` counts an invocation that ended in a panic of `nvs` itself, such as an
internal compiler error. The panic hook increments it once, before the process ends, and records no
message, location or backtrace. A Novis program that throws, fails to compile or exits with an error is
the program's result, not an internal error, and counts nothing. A request that fails inside
`nvs serve` counts nothing either, by the traffic rule below.

The payload is one JSON document, and these are all its fields:

```json
{"schema":1,"version":"0.4.0","os":"linux","arch":"x86_64","week":"2026-W42","first":false,"days_active":3,"counters":{"invocation.run":41,"invocation.serve":2,"internal-error.check":1}}
```

- `os` is one of `linux`, `macos`, `windows` and `other`, and `arch` one of `x86_64`, `aarch64` and
  `other`.
- `week` is the ISO week being reported.
- `days_active` is on how many days of that week, from 1 to 7, `nvs` ran at all. The counter file keeps
  one bit per day of the current week, and only the count leaves it.
- `first` is `true` until the collector has accepted one upload from this data directory. It counts
  new installs per week without any identifier.
- `counters` holds only the counters above zero.

No machine ID, no per-event timestamps, no arguments, no paths, no environment.

**Nothing derived from traffic is ever counted.** `nvs serve` incrementing `invocation.serve` at startup
records the operator's action; request counts, routes, headers and anything else the internet sent belong
to the site's visitors, who never consented, and are permanently out of schema — not merely off by
default. The counter file is provably a function of what the operator did.

- **Memory:** a few KiB in one local file per user, and one buffered append per invocation. For
  `nvs serve` that append happens once, at startup, before the listener binds. Nothing is spent on the
  request path.

**Not on disk.** There is no counter file and no payload.

**Declined:**
- **Counts of compiler diagnostic codes.** They would show which errors confuse people, but they are
  derived from the user's own code, and every other field is derived from what the operator ran.
- **The operating system's version, the CPU count, the memory size, the locale or the time zone.**
  Each one narrows down a machine and answers no question the project has.
- **Which `Core` classes or `nvs.toml` blocks a program uses.** That is a description of the program,
  not of the tool.
