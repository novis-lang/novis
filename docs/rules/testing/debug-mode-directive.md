`[debug] mode` in `nvs.toml` states the default **and** the ceiling in one value: `[]` is off, and
any subset of `coverage`, `branch`, `trace` and `profile` is a ceiling a request may narrow and can
never widen. A production tree sets `mode = []` and no request-side call can turn any bit on.

Whether a running request's internals are observable is not a request-local decision. An
attacker-controlled request that could turn tracing on for itself in production would gain a
reconnaissance channel over call arguments and timing, which is not a trade this language makes.

Writing a trace or a profile is its own capability — `debug.trace` and `debug.profile`, each granted
to named roots, deny-by-default, and **separate from the grant to write a file**. Being able to
write an ordinary file is not permission to persist a continuous log of every call's arguments,
which can carry request data a coverage-only deployment never needed to expose.
