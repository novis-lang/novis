Runs another program, waits until it ends, and returns its exit code and everything it printed.
The printed output is `bytes`, because a program can write anything. Use `as string` for text.

You give the path of the program and an array of arguments, one in each element. The program is
started directly, never through a shell. So a space or a `;` in an argument is only text, and it
cannot start a second command.

The `process.exec` capability in `nvs.toml` lists the folders whose programs may run. A program
outside them, or a `.bat`, `.cmd` or `.ps1` file, throws a `RuntimeError`. A program that cannot be
started, for example because it is missing, throws an `IOError`.

**The examples below** run a program and read its output, show a program that is not allowed, and
check whether a tool succeeded.
