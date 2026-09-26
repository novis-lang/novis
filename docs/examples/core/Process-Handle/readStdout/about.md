Reads the output of a program that `Core\Process::spawn` started, while the program still runs.
Each call waits until the program writes something, and returns it as `bytes`. Use `as string`
to convert it to text.

A call returns one part of the output. A part is whatever has arrived. It can be less than one
line, or many lines. Call `readStdout()` in a loop to read everything. At the end of the output
the result is `null`. Other requests keep running while one request waits.

If you only need the whole output at the end, `Core\Process::run` is simpler.

**The examples below** read a whole output part by part, read from a program that prints
nothing, and pass a program's output on to your own output while the program runs.
