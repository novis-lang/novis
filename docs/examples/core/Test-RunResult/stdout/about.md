`stdout()` returns the text that one run of your program wrote to standard output. In a test,
`Core\Test::run()` runs your own program as a command. The result is a `Core\Test\RunResult`, and
`stdout()` reads the output from it as a `string`.

The result contains everything the program printed with `echo` or `Core\Cli::write`, in order. It
also contains the text printed before `exit`, or before an error the program did not catch. If the
program printed nothing, the result is an empty string. Messages written to standard error are not
part of it. `stderr()` returns those.

A control character, such as a carriage return, is replaced by a visible character before it is
written. The program prints the same text when you start it with `nvs run`.

`stdout()` throws a `RuntimeError` when the output is not valid UTF-8. `bytes()` returns the same
output as `bytes`, and never throws. Each call to `stdout()` returns the same text.

**The examples below** read one line a program printed and give a program some input. The last one
is a test that compares the whole report a command prints.
