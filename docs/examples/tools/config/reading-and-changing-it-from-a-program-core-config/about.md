`Core\Config` reads the configuration and changes some settings for the current request.

A setting's name is its path in the file with dots, such as `limits.hard.memory`. A limit also has a
short name: `memory` is `limits.memory`.

`get` returns the value as a string, or the built-in default. The result is `null` for a setting
with no default, a table or a list.

`set` changes a value for the current request only. Other requests and spawned scripts do not see
it. `set` never throws. It returns `false` and changes nothing when the change is not allowed.
`restore` removes the change for one name. `all` returns every value, sorted by name.

**Good to know:** a program may raise a `Runtime` setting up to its ceiling in `[limits.hard]`. It
may only make a `RuntimeTighten` setting stricter. Only the file sets a `System` setting. Without a
ceiling, a program cannot raise a limit. A command-line program may raise `memory`, unless `[limits.hard]` sets it.
