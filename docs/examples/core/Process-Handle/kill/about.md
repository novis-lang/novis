Stops a program that `Core\Process::spawn` started, at once, whatever the program is doing. Use it
when you do not need the rest of a program's work: for example, you have read enough of its output,
or the program runs too long.

After `kill()`, call `wait()` to get the exit code. A program that was stopped does not return `0`.
On Linux and macOS the exit code is `-1`.

If the program has already ended, `kill()` does nothing and throws no error. So you can call it
without checking first. A program that nobody stops is stopped when the request that started it
ends.

**The examples below** stop a program that never ends, call `kill()` on a program that has already
ended, and read only the first lines of a long output.
