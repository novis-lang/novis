Starts counting how many times each line of the program runs. You read the counts with `Core\Debug::coverage()`.

Counting only works when `nvs.toml` allows it. Add `"coverage"` to the `mode` list under `[debug]`.
The `nvs.toml` beside the examples shows this. With this setting, counting starts when the program starts. `Core\Debug::stopCoverage()` stops it,
and `startCoverage` starts it again. Each new start adds to the counts that are already there.

`startCoverage(true)` also counts each side of every `if` and loop condition. This needs `"branch"`
in `mode` as well. `coverage()` returns only the line counts.

If `mode` does not include the setting a call needs, the call throws a `RuntimeError`. The message
names the line to add. A server never counts lines, whatever the setting says.

**The examples below** count the lines of one method call, show the error when `"branch"` is
missing, and start counting a second time.
