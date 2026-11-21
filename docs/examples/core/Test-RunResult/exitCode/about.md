`exitCode()` returns the exit code of one run of your program. In a test, `Core\Test::run()` runs
your own program as a command, with the words you give it. The result is a `Core\Test\RunResult`,
and `exitCode()` reads the exit code from it.

The exit code is `0` when the program ends normally. When the program calls `exit(3)`, the exit code
is `3`. When the program throws an error that it does not catch, the exit code is `1`. When
`Core\Command::run` prints its usage page, the exit code is `2`. A real shell sees the same codes.

An exit code is a number from `0` to `255`. A number outside that range is changed to fit, as on
Linux and macOS. The exit code is the remainder after division by 256. So `exit(-1)` gives `255`,
and `exit(300)` gives `44`. `exit` with a string prints
the string and gives `0`.

You can call `exitCode()` as often as you like. Each call returns the same number.

**The examples below** show a program that ends normally and a program that calls `exit`. The last
one is a test that checks every way a command-line tool can end.
