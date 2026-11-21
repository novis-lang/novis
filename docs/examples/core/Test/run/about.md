`Core\Test::run()` runs your own program as a command and returns how it ended. Use it to test a
command-line tool.

You give it the words a user would type after the program name, such as `['deploy', 'staging',
'--dry-run']`. You can also give the program its input with `stdin`. The program runs the same way
as with `nvs run`. It runs inside the test's own process, so no new process starts. The result
has the exit code, the standard output and the standard error, each on its own. `exit(3)` in the
program gives the exit code `3`, and the test continues.

When the arguments are wrong, `Core\Command::run` prints a usage page. This is a short text that
shows how to call the command. It is on standard error, and the exit code is `2`. An error the
program does not catch gives the exit code `1`, and its message is on standard error.

**Good to know:** only a test can call `run`. A call inside a program that `run` or
`Core\Test::request` started throws a `LogicError`.

**The examples below** show a command with a flag, a command that reads its input, and a command
called with a missing argument.
