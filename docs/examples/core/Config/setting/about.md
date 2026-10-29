Returns a value your program reads from the `[settings]` block of `nvs.toml`, or `null` if no block sets it.

Use it for values that belong to your program, such as a page size or the address of an API. The
value keeps its type from the file: a `string`, an `int`, a `float`, a `bool` or an array. A table
such as `[settings.api]` is an array with the names as keys. The name `api.base` reads one key of it.

Use `as` to convert the value. `as int` throws an error when the value is missing or is not a whole
number. `as ?int` gives `null`, and `??` then gives a default.

An `[app.settings]` block in an `[[app]]` block replaces values of the same name, for that
application only.

**Good to know:** a setting is not tainted, because the person who runs the server wrote it. Your
program cannot change a setting. Use a variable when one request needs another value. A TOML date
is not allowed, so write it as a string.
