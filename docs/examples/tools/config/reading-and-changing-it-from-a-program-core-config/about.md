`Core\Config` reads the configuration and changes some settings for the current request.

A setting's name is its path in the file with dots, such as `limits.hard.memory`. A limit also
has a short name: `memory` is `limits.memory`.

`get` returns the value as a string. A setting the file does not write gives its built-in default.
The result is `null` for a setting with no default, a table or a list.

`set` changes a value for the current request only. Other requests and spawned scripts do not see
the change. `set` never throws. It returns `false` and changes nothing when only the file may set
the name, when the name is unknown, or when the value is not valid or above the ceiling in
`[limits.hard]`. `restore` removes the change for one name. `all` returns every value
in effect, sorted by name.

**Good to know:** each setting has a class that says who may change it. A program may raise a
`Runtime` setting up to its ceiling and may only make a `RuntimeTighten` setting stricter. Only
the file sets a `System` setting. When `[limits.hard]` does not set a limit, its ceiling is the
value in `[limits]`, so a program cannot raise it. A program you start from the command line may
still raise its `memory` limit, unless `[limits.hard]` sets `memory`.
