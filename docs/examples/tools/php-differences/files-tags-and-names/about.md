How the opening tag, namespaces and imports differ from PHP.

Code in a Novis file starts with `<?nvs`. `<?php` does not compile, and `<?=` works as in PHP. A
file has one namespace, written once as `namespace A;` before any declaration. Each `use` imports
one name, and an import cannot be renamed with `as`. Functions and constants belong to classes, so
`use function` and `use const` do not exist. A name such as `Core\Str::length` is written without a
`\` at the start.

A string cannot name a property at run time. Use an array for keys that are known only at run time.
A string can name a class after you convert it with `as class<T>`. Novis has no magic constants
such as `__DIR__`, `__LINE__` and `PHP_EOL`.

Each PHP form in this section gives a compile error with its own code.

**Good to know:** the short tag `<?` does not start code. Novis copies it to the output as text.
