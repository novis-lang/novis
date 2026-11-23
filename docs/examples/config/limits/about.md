`[limits]` sets the limits that one request or one script starts with. They include the memory it
may use and the CPU time it may spend. They also include the wall time (time on the clock) and the
bytes it may send.

When `[limits]` does not set `memory`, every program may use `128M`. When it does not set
`cpu_time`, a request, a connection, a scheduled script and a queued job may use `30s`. `nvs run`
and `nvs test` have no `cpu_time` limit unless you set one. `false` removes a limit.

A program can lower most of its limits with `Core\Config::set`. Use this to stop a program early
when it uses too much.

On a server, a program can raise a limit only up to the ceiling in `[limits.hard]`. When
`[limits.hard]` does not set a key, the ceiling is the value in `[limits]`. For a value over the
ceiling, `set` returns `false`, and the old value stays. A program you start from the command line
may raise its `memory` limit to any value, unless `[limits.hard]` sets `memory`. A few keys in this
block can be set only in the file, and their own pages say so.
