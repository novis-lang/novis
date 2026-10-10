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

The block is an array-of-tables because an entry is a repeated record with several fields, the shape
`rule:config/lists-are-arrays-and-repeated-records-are-arrays-of-tables` gives every such list and
`[[extension]]` already uses. Keeping the schedule in the root-owned file means a job's existence, its
cadence, its limits and its concurrency policy are reviewed in the same change and deployed by the
same mechanism as the rest of the deployment. The price is that `nvs check` cannot see it: nothing
links an entry to a compiled program, so a renamed script is a boot error rather than a compile
error, and boot-time path checking is what closes most of that gap.

`nvs run jobs/report.nvs` runs the identical file by hand — the whole debugging and backfill story,
and why there is no `--run-now` flag. Durable, retried work is a different mechanism
(`rule:concurrency/queued-work-is-not-scheduled-work`).

**Declined:**
- **A runtime `Core\Schedule::register(cron, callable)`.** It would be process-global state
  established by whichever request ran first, invisible to `nvs check`, and unreachable from a host
  that has not served a request yet — the objections `rule:programs/no-runtime-autoload` makes
  against a runtime autoloader.
- **A `#[Schedule("0 3 * * *")]` attribute discovered at compile time**, the way routes are
  (`rule:routing/routes-are-compiled-not-registered`). A schedule is deployment state, not source
  state: one tree is deployed to staging and production with different cadences, limits and scopes,
  so a config file would override the attribute and the attribute would be a second, weaker copy.
- **Leaving it to system cron.** The schedule would live outside the application, fleet-once would be
  a `flock` guard reinvented per job, and a cron-invoked `nvs run` gets no limits, no capability
  narrowing and nowhere to report a failure but a mail spool.
- **An entry that fires something other than a script**, such as an HTTP request to the server itself
  or a `Core` member. `spawn script` is the one mechanism, and a second one would be a second
  isolation path (`rule:security/isolate-shares-nothing`).
