`Core\Script\ExitReport::reason()` returns how the script ended. The result is a case of the
`Core\Script\ExitReason` enum. `Normal` means the last line of the script ran. `ExitCall` means the
script called `exit`. `UncaughtThrow` means an error reached the top of the script and nothing caught
it. `Finish` means the script called `Core\Script::finish()`.

You read the report inside a function that you added with `Core\Script::onExit()`. Each call to
`reason()` returns the same case. A `FATAL` error and a cancelled script have no case, because they run
no exit function.

**The examples below** show a script that ends after its last line, a script that ends early with
`Core\Script::finish()`, and a job that writes one log line that says how it ended.
