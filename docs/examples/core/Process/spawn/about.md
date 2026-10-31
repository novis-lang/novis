Starts another program and returns at once, while the program is still running. You get a
`Core\Process\Handle`. With it you read the output a part at a time, and you write to the input.
`wait()` waits for the end and returns the exit code.

Use `Core\Process::run` when you only need the output at the end. Use `spawn` for output as it
arrives, or for a program that reads your input.

The program starts directly, never through a shell. Each array element is one argument. A
command-line program may run any program. On a server, the `process.exec` capability in `nvs.toml`
must name the program or its folder. Otherwise `spawn` throws a `RuntimeError`.

Call `wait()` or `kill()` before your script or request ends. A program that still runs then is
stopped, and an error is reported. With `nvs run`, the exit code is `1`. With `nvs serve`, the error
is in the request log.

A program started inside a `Core\Task` child task is stopped when that task ends, and an error is
reported. The exit code does not change. A handle works only in the task that called `spawn`.
