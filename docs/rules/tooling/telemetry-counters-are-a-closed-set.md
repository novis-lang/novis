Any `nvs` invocation may increment counters in the local file — a write, never a connection. A counter is
`invocation.<subcommand>` or `internal-error.<subcommand>`, where `<subcommand>` comes from the binary's
own compiled-in command table, plus exactly one fallback each, `invocation.unknown` and
`internal-error.unknown`. **The literal argv never becomes a counter name**: a typo, a path or a misparse
increments `invocation.unknown` and nothing else, so arbitrary user input cannot leak into the schema,
and every counter that can ever exist is enumerable from the binary that wrote it. A closed set with one
`unknown` bucket is the whole difference between a schema and a log.

The command table is `COMMAND_NAMES` in `crates/nvs-cli/src/main.rs`, and a test holds it equal to the
parser's own list of subcommands. A command line that runs no command, `--help` and `--version`
included, counts `invocation.unknown`. A service's stored line counts the command it runs.

`internal-error.<subcommand>` counts an invocation that ended in a panic of `nvs` itself, such as an
internal compiler error. The panic hook increments it once, before the process ends, and records no
message, location or backtrace. It then calls the hook it wrapped, so the panic prints as before. A
Novis program that throws, fails to compile or exits with an error is the program's result, not an
internal error, and counts nothing. A panic under a request that `nvs serve` runs is claimed by the
request's own log first and counts nothing either, by the traffic rule below.

**The counter file** is `telemetry/counters`, one `key value` line each: `uploads` and `bytes`, the
lifetime totals of accepted uploads; `last`, when the collector last accepted one, and `due`, when the
pending week may be sent, each in seconds since 1970 and absent until it has a value
(`rule:tooling/a-serving-process-never-uploads`); `week`, the current ISO week in UTC, then its `days` as seven
digits from Monday to Sunday and each counter above zero; and `pending`, the last finished week that
counted anything and is not sent yet, laid out the same way. A week that ends becomes `pending`, and
replaces the one there. A week after today, which only a clock set back can leave, is dropped. The file
is read through the closed set, so a line an editor added that names anything else is dropped and never
reaches a payload.

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

- **Memory:** a few KiB in one local file per user, and per invocation one read and one rewrite
  through a file beside it and a rename, so a reader never sees half a file. Two invocations that end
  at the same moment may lose one count. For `nvs serve` the rewrite happens once, at startup, before
  the listener binds. Nothing is spent on the request path.


**Declined:**
- **Counts of compiler diagnostic codes.** They would show which errors confuse people, but they are
  derived from the user's own code, and every other field is derived from what the operator ran.
- **The operating system's version, the CPU count, the memory size, the locale or the time zone.**
  Each one narrows down a machine and answers no question the project has.
- **Which `Core` classes or `nvs.toml` blocks a program uses.** That is a description of the program,
  not of the tool.
