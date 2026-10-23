`nvs update-check`'s output always carries whether telemetry is on or off, the local version and the
latest released version. With telemetry on it adds when the last upload happened, when the next one is
eligible, and the lifetime totals — how many uploads, how many bytes sent — both maintained locally and
never themselves uploaded.

`--json` emits one JSON document with stable, schema-versioned field names for the user's own automation.
The default human output **streams** as it works, one line per step, so the user sees exactly where it is
when something is slow, and follows the terminal's conventions. The exit status is 0 whether or not an
update exists and whether or not a telemetry send succeeded
(`rule:tooling/a-failed-upload-never-fails-the-command`); automation reads the JSON.

**The human output** is these lines, in this order, each written and flushed as it happens:
`telemetry: on, ...` or `telemetry: off, ...` with the upload URL or the note that `NVS_TELEMETRY_URL`
is not set, `this
version: <v>`, `reading the latest release from <url>`, then either `latest version: <v>` and one of
`an update is available: <local> to <latest>` or `you have the latest version`, or `could not read the
latest release: <why>`. The latest version is the Releases API document's `tag_name` with a leading
`v` removed. Versions compare number by number, and a release follows its own pre-release.

**`--json`** prints `{"schema":1,"telemetry":"on"|"off","telemetry_url":…,"local":…,"latest":…,
"update_available":…,"error":…}` in that key order. `latest` and `update_available` are `null` when the
request failed, and `error` is `null` when it did not. The request is one `GET` with a ten-second
bound over the transport `Core\Http\Client` uses, and its reply is capped at 1 MiB.

The switch is on when the data folder's consent is `on` and neither `DO_NOT_TRACK` nor `CI` is set
(`rule:tooling/telemetry-is-two-opt-ins`).

**With telemetry on**, the command carries the upload once the release is read
(`rule:tooling/a-serving-process-never-uploads`). After the release lines it prints the upload's one
line when an upload URL is set, then `last upload: <time>` or `last upload: never`, `next upload: from
<time>` and `uploads accepted: <n>, bytes sent: <b>`. A time is UTC, as `2026-10-26T05:12:00Z`.
`--json` adds `"upload":{"result":…,"message":…}`, where `result` is `sent`, `kept`, `dropped`,
`not-due` or `nothing` and `message` is that line, then `"last_upload"`, `"next_upload"`, `"uploads"`
and `"bytes_sent"`, after `error` and in that order. Each is `null` while telemetry is off, and
`upload` is also `null` when no upload URL is set.
