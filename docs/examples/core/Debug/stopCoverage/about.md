Stops counting how many times each line runs. The counts so far are kept.

After `stopCoverage`, the lines that run are not counted. `Core\Debug::coverage()` still returns
the counts from before the stop. `Core\Debug::startCoverage()` starts counting again and adds to
those counts.

When counting is already off, `stopCoverage` does nothing. It never throws an error, and it needs no
setting in `nvs.toml`.

**The examples below** count only one part of a program, read the counts after a stop, and call
`stopCoverage` when counting is already off.
