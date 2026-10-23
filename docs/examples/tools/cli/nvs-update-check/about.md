`nvs update-check` checks whether a newer version of Novis is released. It prints whether telemetry
is on, the version you run and the latest released version, one line for each step. It reads the
latest release from GitHub. `NVS_UPDATE_URL` replaces that address.

The command always exits with status `0`, also when it cannot connect. A script uses `--json` and
reads the fields `update_available` and `error`. When telemetry is on, the command also sends the last
finished week of counters, if that week is due.

**Good to know:** the example sets `NVS_UPDATE_URL` to a port on your own computer where nothing
listens. It does not connect to GitHub, so its last line says that the release could not be read.
