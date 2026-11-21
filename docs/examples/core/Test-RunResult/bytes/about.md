`bytes()` returns what one run of your program wrote to standard output, as `bytes`. In a test,
`Core\Test::run()` runs your own program as a command. The result is a `Core\Test\RunResult`, and
`bytes()` reads the output from it. The type `bytes` is raw binary data.

The result contains every byte of the standard output, in order. It is the same output that
`stdout()` returns as a `string`. If the program printed nothing, the result is empty `bytes`.

`bytes()` never throws. `stdout()` throws a `RuntimeError` when the output is not valid UTF-8, so
use `bytes()` for such an output. `bytes()` is also useful when you need to count bytes, or compare
the output byte for byte with `==`.

You can call `bytes()` as often as you like. Each call returns the same bytes.

**The examples below** count the bytes of an output and show the bytes as hex digits. The last one
is a test that compares an export with the bytes it expects.
