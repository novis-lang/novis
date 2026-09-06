Scheduled work is a `[[schedule]]` array-of-tables in `nvs.toml`, and nothing else: an entry names a
`cron` expression and a `script`, and the server fires that file as an ordinary `spawn script`
isolate (`rule:security/isolate-shares-nothing`). There is **no API surface** — no `Core\Schedule`,
no registration call, no attribute — because a schedule is deployment state, and a runtime
registration would be process-global state established by whichever request ran first.

```toml
[[schedule]]                          # System, every key
name     = "nightly-report"           # required, unique — the log and metric label
cron     = "0 3 * * *"
script   = "jobs/report.nvs"
scope    = "fleet"                    # required, no default: "fleet" | "host"
timezone = "Europe/Vienna"            # default "UTC"
overlap  = "skip"                     # default "skip": "skip" | "queue" | "kill"
limits   = {memory = "512M", cpu_time = "120s"}   # optional, narrowing only
grants   = {net.connect = ["reports.internal"]}   # optional, narrowing only
```

**Everything an entry must answer is asked at boot, over the merged tree**, and one that cannot
answer refuses the boot as `E0611` naming the entry and the line. `name` is required and unique; a
duplicate names both lines. `script` is resolved against the same `script.spawn` roots a `spawn`
target lives under (`rule:security/script-spawn-capability`), canonicalised and prefix-checked, and a
path outside them — including one reaching out through `..` — is a boot error, not a first-fire one:
the set of files a deployment can execute is one list. Nothing is deferred to the first fire, because
an entry that never fires looks exactly like one whose interval has not come round.

`nvs run jobs/report.nvs` runs the identical file by hand — the whole debugging and backfill story,
and why there is no `--run-now` flag. Durable, retried work is a different mechanism
(`rule:concurrency/queued-work-is-not-scheduled-work`).
