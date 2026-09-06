`nvs update-check`'s output always carries whether telemetry is on or off, the local version and the
latest released version. With telemetry on it adds when the last upload happened, when the next one is
eligible, and the lifetime totals — how many uploads, how many bytes sent — both maintained locally and
never themselves uploaded.

`--json` emits one JSON document with stable, schema-versioned field names for the user's own automation.
The default human output **streams** as it works, one line per step, so the user sees exactly where it is
when something is slow, and follows the terminal's conventions. The exit status is 0 whether or not an
update exists and whether or not a telemetry send succeeded
(`rule:tooling/a-failed-upload-never-fails-the-command`); automation reads the JSON.
