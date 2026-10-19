The `FATAL:` line on standard error names the limit and the numbers. For `memory` it gives the bytes
the program held, the limit, and the part of the limit kept for the handler. With `memory = "16M"`,
1 megabyte is kept for the handler, so the program itself can use 15 megabytes. When `nvs.toml` sets
no `memory` limit, the limit is `128M`.

The handler receives an array with one key, `limit`. Its value is the name of the limit as
`nvs.toml` writes it: `memory`, `cpu_time`, `wall_time` or `max_output`. The handler may also declare
no parameter. A handler with a parameter of another type, such as `fn (int $x): void`, is the error
`E0401`.

`Core\Fatal::onLimit` keeps one handler. A second call replaces the first, so only the last handler
runs.

The handler runs once. If it throws an error, the error is not shown, and the program still writes
the `FATAL:` line and ends with status 1. After the `max_output` limit, what the handler prints with
`echo` still reaches standard output.

Two more errors end the program in the same way: `send` on a closed `Core\Task\Channel`, and
`new Core\Task\Channel<int>(0)`. Each writes a `FATAL:` line that names the method, and the program
ends with status 1. No `catch` and no `finally` runs. The `onLimit` handler does not run either,
because no limit fired.
