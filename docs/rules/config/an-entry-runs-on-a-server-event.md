A `[[schedule]]` entry runs on a server event when it writes `on` in place of `cron`. `on` takes
exactly `"start"`, `"stop"`, `"reload"` and `"failed"`, and an entry names exactly one of `cron` and
`on`. Every other key keeps its meaning — `name`, `script`, `scope`, `overlap`, `limits`, `grants` —
so an event run is the same root isolate a cron run is (`rule:config/a-scheduled-run-is-a-root-isolate`):
the `script.spawn` roots, the `[[app]]` fold, the budget, the narrowing-only grants and the boot
checks apply unchanged, and there is still no runtime API.

```toml
[[schedule]]
name   = "announce-start"
on     = "start"                      # "start" | "stop" | "reload" | "failed"
script = "jobs/announce.nvs"
scope  = "host"                       # the one legal scope for an `on` entry
```

**What the boot refuses, as `E0611` naming the entry and the line:** both `cron` and `on`, neither
of them, an `on` value that is not one of the four, `scope = "fleet"`, and any `timezone`. `scope`
stays required (`rule:config/scope-has-no-default`), and `"host"` is its one legal value here: a
process event happens on one host, so "once for the whole fleet" means nothing for it. A `timezone`
with no `cron` does nothing, and `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`
already refuses a key that does nothing. `nvs_config::schedule::Event::parse` is the one reader of
the four words, called by the boot check and by the ticker's `arm` alike.

**Every scheduled run reads one map from `Core\Script::args()`, cron runs too:**
`["name" => <entry name>, "on" => "cron" | "start" | "stop" | "reload" | "failed"]`, plus the
event's own keys below. One shape for every run is simpler than a string for one trigger and a map
for the other.

- **`start`** runs once per process, on the ticker's first pass. The ticker starts after the
  listeners are bound, so the server is already accepting: a slow or failing start script never
  delays serving and never stops the boot. A reload that adds a `start` entry does not run it.
- **`stop`** runs once when the drain begins, and the drain waits for every stop run up to
  `[server] drain_timeout`, in the same drain the requests are in
  (`rule:concurrency/a-drain-closes-a-connection-cleanly`). A stop run still going when the period
  ends is cancelled as a request is. `stop` never runs on a kill, a crash or a power loss, so a
  "this host is down" alert still needs a monitor outside the process. A `cron` entry due during the
  drain does not run.
- **`reload`** runs once per new reload outcome, under the snapshot serving when it runs. Its map
  adds `"outcome" => "applied" | "refused"` and `"restart" => [<Boot key names>]`, the keys whose
  change waits for a restart. It never carries a value, a file path or the refusal's text, because a
  refusal quotes the line it refused and that line can hold a secret; the script reads the details
  from the log. Several reloads between two passes run it once, with the newest outcome. The ticker
  rearms before it runs events, so an entry the reload itself added runs for that reload.
- **`failed`** runs at most once a minute per entry, with every failure since its last run counted.
  A failure is a scheduled run, cron or event, that threw, reached a limit or did not compile, and a
  queue job this process moved to the dead-letter table. Its map adds
  `"schedule" => [<entry names that failed>]`, each name once, and `"dead_letter" => <int>`. A run
  of a `failed` entry is never counted, so a failing `failed` script cannot run itself again. Under
  `overlap = "skip"` a run still going absorbs the next minute's failures into the run after it.
  Each pass takes the tally out of `Events` and gives every `failed` entry its own copy, so two of
  them both see every failure, and a pass with no `failed` entry drops it, so the tally never grows
  without a reader. A run that `overlap = "kill"` or the drain cancelled is not a failure. The
  ticker records a fire's failure itself, so `nvs serve`'s `Fires` counts nothing.

**The one-minute floor is a constant, not a setting.** A request can cause failures indirectly, by
pushing jobs that fail, and the floor bounds how many runs that can start. One minute is the
resolution `cron` already has.

**The events reach the ticker through one process-wide `Events`**, beside `Armed` in
`nvs_server::schedule`, made once at boot and shared with the reload path and the queue workers. It
holds a reload generation with the newest outcome and the failure tally: a set of failed entry names
and a dead-letter count. The ticker already wakes every `SCHEDULE_POLL`, so an event runs at most one
poll late and no new task, thread or wake exists. Memory: per process, one `Events` with a set no
larger than the roster, and one run per event. Nothing per request.

**Only `nvs serve` runs event entries**, as for every entry. A job dead-lettered by a queue worker
under `nvs run` is logged and runs no `failed` entry.

**Declined:**
- **A separate `[[on]]` or `[[hook]]` block.** It would repeat every key and every boot check of
  `[[schedule]]`.
- **`cron = "@start"`.** The cron dialect is closed (`rule:config/cron-is-five-fields-and-nothing-more`),
  and a shorthand that is not a time would be a second dialect inside it.
- **A scheduler-specific accessor such as `Core\Schedule::event()`.**
  `rule:config/a-scheduled-run-is-a-root-isolate` already declines one; the args map carries the event.
- **A start entry that holds readiness.** A broken webhook would then stop a deployment from serving,
  which spends availability for a notification. Warm-up that must finish before traffic belongs in
  the program's own first request path or a health check.
- **A run per failure.** It turns a burst of failures into a burst of runs at the moment the host is
  already in trouble.
- **A configurable failure window.** Nobody has asked for one, and it is one more key to get wrong.
- **Per-request events** — an error page, an uncaught exception, a request at its memory limit,
  request start or end. An attacker sets their rate and they grow with traffic; `Core\Log` and the
  `[metrics]` and `[trace]` exporters are their home.
- **An event for overload or a stuck core.** A run started then adds load at the worst moment.
- **A deploy or file-change event**, which `reload` already is, and **a "drain started" event**,
  which `stop` already is.
- **A channel per event.** A channel needs a reader task, and the ticker already is one.
