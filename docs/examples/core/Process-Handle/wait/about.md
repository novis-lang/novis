Waits until a program that `Core\Process::spawn` started has ended, and returns a
`Core\Process\Result`. The result has the exit code, the output and the error output of the
program.

`wait()` first closes the standard input of the program. So a program that reads until its input
ends can finish. If you already read part of the output with `readStdout()` or `readStderr()`, the
result has only the part that you did not read. While `wait()` waits, the server keeps handling
other requests.

If the output is larger than `[limits] max_output`, `wait()` stops the program and throws a
`RuntimeError`. You can call `wait()` a second time. It returns the same exit code and empty
output.

**The examples below** read an exit code and an error message, show the output that was already
read, and run three programs at the same time.
