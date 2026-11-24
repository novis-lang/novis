`Core\Test::run(array<string> $arguments, {stdin?: string|bytes}): Core\Test\RunResult` runs the
program's entry, the file `nvs run` runs, in a child isolate, as `Core\Test::request` runs a request
(`rule:testing/in-process-request`). `$arguments` is the command line after the program name, so a
command program's words reach the command they name, and `stdin` is its whole input; without it the
input is empty.

`Core\Test\RunResult` has `exitCode(): int`, `stdout(): string`, `stderr(): string` and `bytes():
bytes` for stdout, the names `Core\Process`'s result uses (`rule:core-classes/process-run`). The two
captures stay apart. `exit(n)` in the child ends the child, never the test, and `n` is its exit code.
An uncaught error is the exit code `nvs run` gives for one, with its message on stderr, and a usage
page a command program prints is on stderr with its exit code, as on a terminal.

The child runs on the test's clock and environment, a copy of each taken when the call starts
(`rule:testing/task-tree-and-virtual-clock`, `rule:testing/test-attribute`), with the run's
`[capabilities]`, as a test request does. A unit with no entry `nvs run` would run is a
`LogicError`. A run from inside a run or a request is refused, as a request from inside a request is.
What it spends, per call: the child's isolate and both captures, held until the result is freed.

**Declined:**
- **A `workingDirectory` option.** A program reads no working directory but its grants, so the option would set
  nothing the program can observe.
- **A fake for `Core\Process`.** A program that runs a command wraps it in its own class, and
  `Core\Test::double` replaces that class (`rule:testing/doubles`).
