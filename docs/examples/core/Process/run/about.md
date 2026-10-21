Runs another program, waits until it ends, and returns its exit code and everything it printed.
The printed output is `bytes`, because a program can write anything. Use `as string` for text.

You give the name or the path of the program and an array of arguments, one in each element. A name
such as `git` is looked up in the folders of `PATH`. The program is started directly, never through
a shell. So a space or a `;` in an argument is only text, and it cannot start a second command.

A program you start from the command line may run any program. On a server, the `process.exec`
capability in `nvs.toml` must name the program or its folder. Otherwise `run` throws a
`RuntimeError`. A `.bat`, `.cmd` or `.ps1` file always throws a `RuntimeError`. A program that cannot be
started, for example because it is missing, throws an `IOError`.

**The examples below** run a program and read its output, show a program that is not allowed, and
check whether a tool succeeded.
