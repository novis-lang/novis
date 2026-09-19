A resource limit stops your program at once, and no `catch` sees it.

`nvs.toml` sets four limits for a program: `memory`, `cpu_time`, `wall_time` and `max_output`. A
program that goes past one of them is stopped where it stands. This is a `FATAL`. It is not a
`Throwable`, so no `catch` clause matches it and no `finally` runs. The program writes a line starting
with `FATAL:` to standard error and ends with status 1.

One hook runs before that end. `Core\Fatal::onLimit` registers a function that runs once when a limit
stops the program, and it is told which limit fired. Use it to write a last note about what the
program was doing. Registering a second function replaces the first. The program still ends after the
handler.

**The examples below** show a limit that no `catch` and no `finally` sees, and a handler that names
the limit that fired. The third is a job that records the order it stopped on.
