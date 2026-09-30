The ten most important differences between PHP and Novis.

Every variable, parameter and property declares its type once, and a value never changes its type.
Every function is a method of a class, and every constant belongs to a class. Novis has no global
variables, so `$_GET` and `global` do not exist. The methods of the `Core` classes replace PHP's
built-in functions, and a failure throws an error.

`==` never converts its operands, and `===` does not exist. A `string` is UTF-8 text, and binary
data has the type `bytes`. The keys of an array are always strings. Novis has no `eval`, no
references and no magic methods.

Tasks and channels for work that runs at the same time are built in. A program needs a permission
in `nvs.toml` to read a file or to open a network connection. Tests are part of the language, and
`nvs test` runs them.
