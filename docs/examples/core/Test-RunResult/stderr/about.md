`stderr()` returns the text that one run of your program wrote to standard error. In a test,
`Core\Test::run()` runs your own program as a command. The result is a `Core\Test\RunResult`, and
`stderr()` reads the standard error from it as a `string`.

A program writes a message to standard error with `Core\Cli::write` and
`{ stream: Core\Cli::Stream::Err }`. Two other messages also go there. The first is an error the
program does not catch: it starts with `[error]` and the message, and the lines after it show where
the error was thrown. The second is the usage page that `Core\Command::run` prints for a wrong
command name. If the program wrote nothing to standard error, the result is an empty string.

The text that the program printed with `echo` is not part of it. `stdout()` returns that text. So a
test can check the result and the messages one at a time.

`stderr()` throws a `RuntimeError` when the text is not valid UTF-8. Each call returns the same text.

**The examples below** show an error the program does not catch and keep progress notes apart from
the result. The last one is a test that checks that a typo prints the usage page.
