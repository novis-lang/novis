Reads the error output of a program that `Core\Process::spawn` started, while the program still
runs. Programs write their error messages and warnings to this output, which is called standard
error. Each call waits until the program writes something there, and returns it as `bytes`. Use
`as string` to convert it to text.

It works like `readStdout`. The two outputs are separate, so an error message never appears in
what `readStdout` returns. Call `readStderr()` in a loop to read everything. At the end of the
error output the result is `null`.

**The examples below** read every warning a program writes, keep a result apart from a note
about it, and show why a job failed.
