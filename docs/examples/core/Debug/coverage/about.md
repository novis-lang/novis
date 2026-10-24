Returns how many times each line of the program ran.

The result is an array with one entry for each file. The key is the file path. The value is an
array from a line number to the number of times that line ran. A line that never ran is not in the
result. So the result grows with the code that ran, not with the size of the program.

`nvs run` counts lines when `nvs.toml` has `"coverage"` in the `mode` list under `[debug]`. The
`nvs.toml` beside the examples shows this. With this setting, counting starts when the program
starts. `Core\Debug::stopCoverage()` and `Core\Debug::startCoverage()` stop and start it. `coverage` returns the counts at any time, also
while counting is off. Without the setting, the result is an empty array.

**The examples below** list every line that ran, check whether one line ran, and count the calls of
a method.
