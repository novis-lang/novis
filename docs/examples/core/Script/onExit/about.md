`Core\Script::onExit()` adds a function that runs when the script ends. It runs after the last line of
the script, after `exit`, after `Core\Script::finish()` and after an error that nothing caught. If you
add several functions, they run in the order you added them, and each one runs once.

Each function receives a `Core\Script\ExitReport`. The report says how the script ended, which exit
status the process will have, and which error ended it, if there was one. A function cannot change the
ending. If a function throws an error, the error is logged and the next function still runs.

Two endings run no function: a `FATAL` error, such as a script that goes over its memory limit, and a
script that was cancelled.

**The examples below** show two functions that run after the last line, a function that runs after
`exit`, and a job that releases its lock when an error ends it.
