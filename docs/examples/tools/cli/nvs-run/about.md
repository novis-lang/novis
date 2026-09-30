`nvs run <file>` checks a program, compiles it and runs it.

The command first checks the file the same way as `nvs check`. If there is an error, it prints the
error and stops. The program runs with the current directory as its working directory. Before the
run, `nvs` reads `./nvs.toml` if that file exists. `--config <path>` reads the named file, and
`./nvs.toml` is then not read. Repeat the option to read several files in order.

The exit status is `0` when the program ends normally, and `n` when the program calls `exit(n)`. It
is `1` for a compile error, an error that nothing caught, or a limit that was reached. It is `2`
for a wrong command line.

**Good to know:** `exit("message")` prints the message, and the exit status is `0`. An error that
nothing caught is written to standard error and never to standard output.
