Returns everything a program wrote to its standard error. Programs write their error messages and
warnings there, and their normal results to the standard output. The two outputs are kept apart, so
a warning never ends up in the middle of the data that `stdout` returns.

The result is `bytes`, like the result of `stdout`. Use `as string` to convert it to text. When the
program wrote no errors, the result is empty.

A good time to read this output is when `exitCode` is not `0`. The message usually says what went
wrong, so you can put it in your own error or in a log.

**The examples below** read the two outputs of one program, check that a program wrote no
warnings, and put a failed tool's message in an error.
