Returns the exit code of a program that has ended. A program sets this number when it ends, to say
whether its work succeeded. By convention, `0` means success and any other number means failure.
Many tools give each kind of failure its own number, and their manual lists them.

On Linux and macOS, a program can also be stopped by a signal before it sets an exit code, for
example after `kill`. The exit code is then `-1`. On Windows, a stopped program has an exit code
that is not `0`.

The program has already ended when you have a `Core\Process\Result`, so `exitCode` never throws an
error. It returns the same number every time you call it.

**The examples below** tell success from failure, read the exit code of a program that was
stopped, and run a list of checks before a job starts.
