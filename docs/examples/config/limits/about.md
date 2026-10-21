What one request or one script starts with: the memory it may use, the CPU time and wall time it may
spend, the bytes it may write back, and its other limits.

When `[limits]` does not set `memory`, every program may use `128M`. When it does not set
`cpu_time`, a request, a connection, a scheduled script and a queued job may use `30s`. `nvs run`
and `nvs test` have no `cpu_time` limit unless you set one. `false` removes a limit.

A program can lower its own limits with `Core\Config::set`. A limit a program set for itself still
stops it. This stops a runaway program early.

On a server, a program can raise a limit only up to the ceiling in `[limits.hard]`, which the
person who runs the server sets. When `[limits.hard]` does not set a key, the ceiling is the value
in `[limits]`. A value over the ceiling returns `false`, and the previous value stays in force.
A program you start from the command line may raise its `memory` limit to any value, unless
`[limits.hard]` sets `memory`. A few keys in this
block can be set only in the file, and their own pages say so.
