Sends data to a program that `Core\Process::spawn` started, while the program runs. The program
reads the data from its standard input, the same way it reads what a person types. The call waits
until the program has taken all of the data.

You can call `writeStdin()` many times. The input stays open between the calls. `wait()` closes the
input, so a program that reads until its input ends can then finish. The data is `bytes`. Use
`as bytes` to convert a `string`. A `tainted` value from a user is allowed, because the program
reads it as data and does not run it as a command.

**The examples below** give a list to `sort`, talk to a program one line at a time, and keep only
the error lines of a log.
