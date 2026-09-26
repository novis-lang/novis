Starts another program and returns at once, while the program is still running. You get a
`Core\Process\Handle`. With it you read what the program prints, a part at a time, and you write to
its input. `wait()` then waits for the end and returns the exit code.

Use `Core\Process::run` when you only need the output at the end. Use `spawn` when you want the output
while it arrives, or when the program reads input from you.

The program is started directly, never through a shell, with one argument in each array element. The
`process.exec` capability in `nvs.toml` must allow it, or `spawn` throws a `RuntimeError`. The
program is stopped when the request that started it ends.

**The examples below** read output as it arrives, show a program that is not allowed, and send lines
to `sort` and read them back sorted.
