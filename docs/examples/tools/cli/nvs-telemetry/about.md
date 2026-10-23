`nvs telemetry` controls the usage counters of `nvs`. Telemetry is off until you run
`nvs telemetry on`. After that, each `nvs` command adds 1 to a counter for that command. The counters
are in a plain text file in the data folder. They contain no paths, no arguments and no text from your
programs.

`nvs telemetry status` prints whether telemetry is on. `nvs telemetry show` prints the exact JSON
document that an upload sends. Nothing is uploaded unless `NVS_TELEMETRY_URL` is set.

If `DO_NOT_TRACK` or `CI` has a value that is not empty, a command records nothing and uploads
nothing. Your setting stays the same.

**Good to know:** the examples use `--data data`. They do not change the settings of your own `nvs`.
