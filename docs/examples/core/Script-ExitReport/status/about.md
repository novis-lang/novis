`Core\Script\ExitReport::status()` returns the exit status of the process, as a whole number. A status
of `0` means success. After a normal end and after `Core\Script::finish()`, the status is `0`. After
`exit($n)`, the status is `$n`. After an error that nothing caught, the status is `1`.

You read the report inside a function that you added with `Core\Script::onExit()`. The status is fixed
before the first function runs, and a function cannot change it.

**The examples below** show the status after the last line, the status after an error that nothing
caught, and a job that writes its status in a log line for a monitoring tool.
