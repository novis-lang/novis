`Core\Debug::render` returns a `Core\Cli\Text`, not a `string`. `echo` prints it. Call `->text()` to
get the `string`, for example to compare it: `Core\Debug::render(1)->text() == "int(1)"` is `true`.
Two `Core\Cli\Text` values are objects, so `==` on them compares the objects, not the text.

Each value shows its type. `null` is `null`, and a number is `int(1)`, `uint(3)`, `float(1.5)` or
`decimal(2.50)`. An enum case shows its number, such as `int(1)`. A function shows the number of its
parameters, such as `callable(1 parameter)`. A string shows its length in bytes, so `"café"` is
`string(5)`.

A newline is shown as `\n` and a tab as `\t`. Other control bytes and the characters that change the
direction of text are shown as visible symbols.

An array shows its first 100 entries, then `[elided: 200 of 300 entries]` for the rest. A string over
1024 bytes shows its first 1024 bytes, as `string(1024+) "…" [elided: 3976 bytes]`. A structure shows
eight levels, and a deeper value is `[elided: below the depth cap]`. An object that was already shown
higher up is `[cycle -> #1]`, with the number of that object.

`Core\Debug::dump()` with no value writes nothing. Output from `dump` inside `Core\Out::capture` is
not captured. It still goes to standard error.

A `secret` property is shown as `[redacted]`. A `secret` variable cannot be passed to `dump` or
`render` at all: that is the error `E0724`.

Inside a web request, `dump` writes a log record at the `Debug` level. In production, that record is
kept only when `[log] level` is `"Debug"`.
