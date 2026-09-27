`Core\Script\ExitReport::error()` returns the error that ended the script. There is one only when the
reason is `UncaughtThrow`: an error reached the top of the script and nothing caught it. For every other
ending, the result is `null`. An error that a `catch` block caught does not end the script, so it is
not in the report.

The result is the same object that the script threw. It has its own class, its `message` and its
backtrace, and you can test its class with `is`.

**The examples below** show a script with no error, an error that nothing caught, and a job that writes
an alert with the message of the error.
