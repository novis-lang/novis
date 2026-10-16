How long a worker may stay past the deadline of its oldest request before the server reports that
the worker has stopped. A worker is the thread that accepts and runs requests on one CPU core.

A request that runs too long is stopped by its own time limit. This setting is for a worker that
does not run at all any more, for example because of a bug in the engine. The server then writes
one line to its error output. The line names the CPU of that worker.

When the file does not set it, the margin is 5 seconds. The value is a duration, such as `"5s"` or
`"500ms"`. A bare number is seconds. The values `0` and `false` are not allowed, and the server does
not start with either of them.

The server reads this setting when it starts, so a new value takes effect only after a restart.
Until then, the server writes the new value to its log once, in a `configuration restart pending`
line. A program cannot change this setting.
