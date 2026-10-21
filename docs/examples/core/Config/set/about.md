Changes one setting for this request.

No other request sees the change, and it is gone when the request ends. The name is the same as for
`Core\Config::get`. The value is written as in a configuration file: `512M`, `30s`, `64`.

The result is `true` when the change was made. It is `false` when the change is not allowed, and the
old value stays. This happens for a setting only the server may set, an unknown name, a value in the
wrong format, or a value above the ceiling. `set` throws no error, so check the result.

The ceiling of a limit is in `[limits.hard]`. Without it, the ceiling is the starting value, so a
program can lower that limit but not raise it. A command-line program may raise `memory` to any
value, unless `[limits.hard]` sets `memory`.

**Good to know:** `Core\Config::restore` puts back the value from the configuration file, or the
built-in default.
