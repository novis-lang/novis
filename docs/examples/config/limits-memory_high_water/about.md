A warning in the log when a request comes close to its memory limit.

Every request has a memory limit, `[limits] memory`. This setting is a fraction between `0.0` and
`1.0`. When a request's highest memory use is more than that part of its limit, the server writes
one `Warn` record to `Core\Log` at the end of the request. The record names the highest memory use,
the limit and the route. With `memory = "256M"` and `memory_high_water = 0.8`, a request that used
more than 80% of 256 MiB writes the warning.

The setting is off by default, and then nothing is logged. A value of `0` is not off: every
request writes the warning. A value below `0.0` or above `1.0` is an error, and the server does not
start.

Only the person who runs the server sets this value. A program cannot change it, and
`Core\Config::set` returns `false`.

The example reads the setting, which is not set, and then tries to change it. Both changes fail.
