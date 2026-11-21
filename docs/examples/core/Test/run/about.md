`Core\Test::run()` runs your own program as a command and returns how it ended. Use it to test a
command-line tool.

You give it the words a user would type after the program name, such as `['deploy', 'staging',
'--dry-run']`. You can also give the program its input with `stdin`. The program runs the same way
as with `nvs run`, but no new process starts. The result has the exit code, the standard output and
the standard error, each on its own. `exit(3)` in the program gives the exit code `3`, and the test
goes on.

A usage page from `Core\Command::run` is on standard error, with the exit code `2`. An error the
program does not catch gives the exit code `1`, and its message is on standard error.

**Good to know:** a program that `run` started cannot call `run` again. That call throws a
`LogicError`.

**The examples below** show a command with a flag, a command that reads its input, and a command
called with a missing argument.
