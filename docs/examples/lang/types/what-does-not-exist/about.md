This page lists PHP syntax that Novis does not have, and what to write instead.

There are no casts such as `(int)$x`. Write `$x as int`. The conversion is checked. If the value
does not fit, `as int` throws an error, and `as ?int` gives `null`.

There are no free functions, such as `intval`, `gettype` or `is_int`. To test what a `mixed` value
is, use `is` for a class, `== null` for `null`, and `($x as ?int) != null` for a number.

A string is not an array of characters. `$s[0]` does not compile, so use `Core\Str::at` or
`Core\Str::slice`. A string is never part of arithmetic, and `<` does not compare two strings. Use
`Core\Str::compare` to order them.

To take an array apart, write `[int $a, int $b] = $pair;`, with a type for each variable.

Each of these mistakes is a compile error, so you see them all before the program runs.
