The PHP operators that Novis does not have, and what to write for each one.

`and` and `or` are written `&&` and `||`. `===` and `!==` are written `==` and `!=`, which never
convert their operands. `<>` is written `!=`. `+` does not join two arrays, and
`Core\Arr::underlay` does. `$s[0]` does not read a character of a string. Use `Core\Str::slice` or
`Core\Str::at`. A type test is written `$x is A`.

Novis has no references. A parameter that the function changes is declared with `inout`, and the
call writes `inout` too. `@` and the backtick operator do not exist. A failure throws an error, and
you catch it.

**Good to know:** `<=>`, `**`, `??`, `??=`, `?:`, `?->` and `.=` work as in PHP.
