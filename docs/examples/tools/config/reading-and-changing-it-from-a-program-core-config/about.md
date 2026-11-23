`Core\Config` reads the configuration and changes some settings for the current request.

A setting's name is its path in the file with dots, such as `limits.hard.memory`. A limit also has a
short name: `memory` is `limits.memory`.

`get` returns the value as a string, or the built-in default. The result is `null` for a setting
with no default. It is also `null` for a name whose value is a table or a list.

`set` changes a value for the current request only. Other requests and spawned scripts do not see
it. `set` never throws. It returns `false` and changes nothing when the change is not allowed.
`restore` removes the change for one name. `all` returns every value, sorted by name.

`set` changes only the settings in the list on the page of `Core\Config::set`. Only the file sets
every other setting. The compiler checks a name written as text in the call. If `set` may never
change that setting, the program does not compile.

**Good to know:** a limit's ceiling is the highest value a program may set. It is the limit's
value in `[limits.hard]`. When `[limits.hard]` does not set the limit, a server program cannot
raise it. A command-line program is different: it may raise `memory` to any value.
