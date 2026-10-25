The type of `error` is `null|{ class: string, message: string }`. It is `null` when `ok` is `true`. So
`$r->error->message` is `E0459`. Read it with `?->`, or test `$r->error != null` first.

`error` has two fields, `class` and `message`. It has no `backtrace` and no `line`. `$r->error?->line`
compiles, and throws a `RuntimeError` when it runs: `` `$shape{class,message}` has no field `line` ``.

`class` is the name of the class the child threw. For a class the child declares itself, such as
`ImportFailed`, the result has that name. A `RecursionError` in the child gives the class
`RecursionError` and the message `the call stack is too deep`. A division by zero gives
`ArithmeticError`.

A failure in the child never throws in the parent. A `try` around `await` catches nothing, and `ok`
is `false`. With `output: "inherit"`, what the child printed is printed at `await`, and then `ok` is
`false` too.

A child that calls `exit(3)` gives `ok` `true` and `error` `null`. The result has no status number.

A missing file and a file that does not compile give the same message in the parent:
`` `spawn script '<path>'`: `<path>` could not be compiled; see the errors above ``. The compiler's
errors are printed to standard error. For a missing file, that is a `could not read` line with the
message of the operating system.

Without the grant, the message is `` `spawn script` needs the capability `script.spawn` for <path>,
which is not granted ``. A second line starts with `help:` and names `[capabilities.script]` in
`nvs.toml`.
