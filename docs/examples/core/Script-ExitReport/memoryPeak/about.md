`Core\Script\ExitReport::memoryPeak()` returns the most memory the script used while it ran, in bytes.
It is the same number `Core\Budget::memoryPeak()` returns, read once when the script ends.

You read the report inside a function that you added with `Core\Script::onExit()`. The number is read
before the first function runs. Memory that a function uses after that does not change it, so every
function gets the same number.

Memory that the script already freed is still part of the peak. A script that loaded a large file
and kept only a short summary has a small figure for `Core\Budget::memoryHeld()` at the end, and a
large peak.

**The examples below** show a peak that includes freed memory, a peak that does not change inside an
exit function, and a job that writes its peak in a log line.
