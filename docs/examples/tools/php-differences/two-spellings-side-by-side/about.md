One short program in PHP syntax and the same program in Novis syntax, so you can compare them.

The PHP version has a function outside a class, a parameter without a type, `array_sum`, `===` and
`and`. It does not compile in Novis. The error is `E0215`, for the function outside a class.

The Novis version puts the function and a constant in a class named `Cart`. It declares
`array<int> $prices`, converts a string with `as int`, and uses `&&`. It also shows
`Core\Str::slice`, `<=>`, destructuring with types, a closure written with `fn`, and a `catch`
clause that reads `$e->message`.

Two smaller PHP programs show two more errors. `__construct` gives `E0114`, and `$_GET` gives
`E0211`.
