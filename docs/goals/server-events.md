---
looper: 1
title: a schedule entry runs on a server event
summary: "Reached when a [[schedule]] entry with on = start, stop, reload or failed runs its script on that event under nvs serve, is checked at boot, and has its feature proofs."
context: [docs/rules/config/scheduled-work-is-a-config-block.md, docs/rules/config/a-scheduled-run-is-a-root-isolate.md, crates/nvs-config/src/tree.rs, crates/nvs-server/src/schedule.rs]
stages:
  - id: 1
    title: the on key is checked at boot
    status: open
    context: [crates/nvs-config/src/schedule.rs, crates/nvs-config/src/directive.rs, crates/nvs-config/src/default.toml, crates/nvs-config/tests/resolve.rs, docs/rules/config/cron-is-five-fields-and-nothing-more.md, docs/rules/config/scope-has-no-default.md, data/rules/config.json]
  - id: 2
    title: start and stop entries run, and every run reads one map
    status: open
    context: [crates/nvs-cli/src/serve.rs, crates/nvs-stdlib/src/script.rs, docs/rules/concurrency/one-process-serves-requests-schedules-and-jobs.md, docs/rules/concurrency/a-drain-closes-a-connection-cleanly.md]
  - id: 3
    title: reload and failed entries run
    status: open
    context: [crates/nvs-cli/src/serve.rs, crates/nvs-cli/src/reload.rs, crates/nvs-cli/src/worker.rs, docs/rules/config/a-reload-names-what-it-could-not-apply.md, docs/rules/concurrency/attempts-are-finite-and-a-dead-letter-is-kept.md]
  - id: 4
    title: the server runs every event end to end, and the feature has its proofs
    status: open
    context: [crates/nvs-cli/tests/live_config.rs, docs/examples/config/schedule/about.md, docs/examples/config/schedule/01-work-that-runs-on-a-clock.nvs, tests/hostile/config/schedule/01-scheduling-my-own-work-from-a-request.nvs, website/src/content/docs/guides/simple-programs/scheduled-job.mdx, website/src/content/docs/guides/cookbook/schedule-job.mdx]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: an-event-entry-is-checked-at-boot
    stage: 1
    name: "an entry with on is refused at boot as E0611 unless it names exactly one of cron and on, a known event, scope host and no timezone"
    run: [cargo, test, --test, resolve, --, an_event_entry_is_checked_at_boot, an_entry_names_exactly_one_of_cron_and_on]
    timeout: 1800
    tests:
      - an_event_entry_is_checked_at_boot
      - an_entry_names_exactly_one_of_cron_and_on
  - id: start-and-stop-fire
    stage: 2
    name: "the ticker runs each start entry once on its first pass and each stop entry once on the drain, waits for the stop runs, and never runs an event entry on a minute"
    run: [cargo, test, --lib, --, a_start_entry_runs_once_on_the_first_pass, a_stop_entry_runs_on_the_drain_and_is_waited_for, a_minute_never_runs_an_event_entry, every_scheduled_run_reads_one_map_from_its_args]
    timeout: 1800
    tests:
      - a_start_entry_runs_once_on_the_first_pass
      - a_stop_entry_runs_on_the_drain_and_is_waited_for
      - a_minute_never_runs_an_event_entry
      - every_scheduled_run_reads_one_map_from_its_args
  - id: reload-and-failed-fire
    stage: 3
    name: "a reload entry runs once per new reload outcome, and a failed entry runs at most once a minute with every failure since its last run counted and its own failures left out"
    run: [cargo, test, --lib, --, a_reload_entry_runs_once_per_new_outcome, a_failed_entry_runs_at_most_once_a_minute_and_counts_every_failure, a_failed_entry_never_counts_its_own_failure]
    timeout: 1800
    tests:
      - a_reload_entry_runs_once_per_new_outcome
      - a_failed_entry_runs_at_most_once_a_minute_and_counts_every_failure
      - a_failed_entry_never_counts_its_own_failure
  - id: events-run-under-nvs-serve
    stage: 4
    name: "under the built nvs serve, a start, stop, reload and failed entry each run their script on their event"
    run: [cargo, test, --test, schedule_events, --, a_start_and_a_stop_entry_run_under_nvs_serve, a_reload_entry_runs_after_a_saved_configuration, a_failed_entry_runs_after_a_scheduled_script_throws]
    timeout: 1800
    tests:
      - a_start_and_a_stop_entry_run_under_nvs_serve
      - a_reload_entry_runs_after_a_saved_configuration
      - a_failed_entry_runs_after_a_scheduled_script_throws
  - id: schedule-proofs-complete
    stage: 4
    name: "the schedule directive owes no feature proof"
    run: [bun, nv, proofs, --id, "directive:schedule"]
    timeout: 600
---

# a schedule entry runs on a server event

## Why

A developer often wants to run code when the server starts or stops: to send a notification, to
warm a cache, or to tell a monitor that a host is down for maintenance. Novis has nothing for this
today:

- `[[schedule]]` runs a script on a clock only. `cron` is five fields and five shorthands
  (`rule:config/cron-is-five-fields-and-nothing-more`), and none of them is an event.
- `Core\Signal::onShutdown` runs a function inside one request. The function belongs to that
  request and is deleted when the request ends, so with no request in flight nothing runs
  (`crates/nvs-stdlib/src/signal.rs:88`).
- `Core\Server::isDraining()` reports a shutdown and runs nothing.
- The lifecycle records (`rule:packaging/a-service-answers-its-manager`) are for the operator to
  read, and run no user code.

Two more events need somebody watching them today. A reload that was refused, or that changed a
`Boot` key nobody restarted for, is one `Core\Log` record
(`rule:config/a-reload-names-what-it-could-not-apply`). A queue job that used up its attempts moves
to the dead-letter table, and `rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept` itself
says that an unwatched dead-letter table is how a queue loses work silently. A scheduled run that
throws is one line on standard error (`crates/nvs-cli/src/serve.rs:2186`).

The user asked for start and stop, and then agreed to the four events below
(2026-11-11, interactive session).

## Standing decisions

**An event is a `[[schedule]]` entry with `on` in place of `cron`.** `on` takes exactly
`"start"`, `"stop"`, `"reload"` and `"failed"`. An entry names exactly one of `cron` and `on`.
Every other key keeps its meaning: `name`, `script`, `scope`, `overlap`, `limits`, `grants`. So an
event run is the same root isolate a cron run is (`rule:config/a-scheduled-run-is-a-root-isolate`):
the `script.spawn` roots, the `[[app]]` fold, the budget, the narrowing-only grants and the boot
checks all apply unchanged, and there is still no runtime API. Declined: a separate `[[on]]` or
`[[hook]]` block, because it would repeat every key and every boot check of `[[schedule]]`.
Declined: `cron = "@start"`, because the cron dialect is closed and a shorthand that is not a time
would be a second dialect inside it. Safe default if `Schedule` cannot take a second trigger
cleanly: an `enum Trigger { Cron(Cron), On(Event) }` on `Armed`, built once by `arm`.

**What `E0611` refuses for an `on` entry, at boot, naming the entry and the line:** both `cron`
and `on`, neither of them, an `on` value that is not one of the four, `scope = "fleet"`, and any
`timezone`. `scope` stays required (`rule:config/scope-has-no-default`), and `"host"` is its one
legal value with `on`: a process event happens on one host, so "once for the whole fleet" has no
meaning for it. A `timezone` with no `cron` is a key that does nothing, and
`rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one` already refuses a key that does
nothing.

**Every scheduled run reads one map from `Core\Script::args()`, cron runs too.** The map is
`["name" => <entry name>, "on" => <"cron" | "start" | "stop" | "reload" | "failed">]`, plus the
event's own keys below. Today a cron run gets its name as a string (`crates/nvs-cli/src/serve.rs:2163`).
This changes what a cron script reads, which is a break, and Novis is pre-alpha. One shape for every
run is simpler than a string for one trigger and a map for the other. Declined: a scheduler-specific
accessor such as `Core\Schedule::event()`, which `rule:config/a-scheduled-run-is-a-root-isolate`
already declines.

**`start` runs once per process, on the ticker's first pass, and does not hold readiness.** The
ticker runs after the listeners are bound (`crates/nvs-cli/src/serve.rs:1464`), so the server is
already accepting. A slow or failing start script never delays serving and never stops the boot.
A reload that adds a `start` entry does not run it, because the process did not start. Declined: a
start entry that gates readiness, because a broken webhook would then stop a deployment from serving,
which spends availability for a notification. Warm-up work that must finish before traffic belongs
in the program's own first request path or in a health check, and that is not this goal.

**`stop` runs once when the drain begins, and the drain waits for it up to `[server] drain_timeout`.**
Today the ticker breaks out on `is_draining` (`crates/nvs-cli/src/serve.rs:1526`). It now runs each
`stop` entry first and waits for every fire it started, in the same drain the requests are in
(`rule:concurrency/a-drain-closes-a-connection-cleanly`). A stop run still going when the drain
period ends is cancelled as a request is. `stop` never runs on a kill, a crash or a power loss, and
the rule and the guide say so in one sentence: a "this host is down" alert still needs a monitor
outside the process. A cron entry that is due during the drain does not run, as today.

**`reload` runs once per new reload outcome, under the snapshot that is serving when it runs.**
Its map adds `"outcome" => "applied" | "refused"` and `"restart" => [<Boot key names>]`, the keys
whose change waits for a restart. It never carries a value, a file path or the refusal's text,
because a refusal quotes the line it refused and that line can hold a secret. The script reads the
details from the log. Several reloads between two passes run it once, with the newest outcome.
The ticker rearms before it runs events, so an entry the reload itself added runs for that reload.

**`failed` runs at most once a minute per entry, with every failure since its last run counted.**
A failure is a scheduled run (cron or event) that threw, reached a limit or was not run because its
script did not compile, and a queue job this process moved to the dead-letter table. Its map adds
`"schedule" => [<entry names that failed>]`, each name once, and `"dead_letter" => <int>`. A run of
a `failed` entry is never counted, so a failing `failed` script cannot run itself again. The
one-minute floor is a constant, not a setting: a request can cause failures indirectly by pushing
jobs that fail, and the floor bounds how many runs that can start. One minute is the resolution
`cron` already has. Under `overlap = "skip"` (the default) a run still going absorbs the next
minute's failures into the run after it. Declined: a run per failure, because that turns a burst
of failures into a burst of runs at the moment the host is already in trouble. Declined: a
configurable window, because nobody has asked for one and it is one more key to get wrong.

**The events reach the ticker through one process-wide `Events`, polled every `SCHEDULE_POLL`.**
`Events` lives in `crates/nvs-server/src/schedule.rs` beside `Armed`, is made once at boot in
`serve.rs` and shared as an `Arc` with the reload path and the queue workers. It holds a reload
generation with the newest outcome, and the failure tally: a set of failed entry names and a
dead-letter count. The ticker already wakes every `SCHEDULE_POLL` (`crates/nvs-cli/src/serve.rs:2076`),
so an event runs at most one second late and no new task, thread or wake exists. **What it spends:**
per process, one `Events` with a set no larger than the roster, and one run per event. Nothing per
request, and nothing at all for a tree with no `on` entry beyond the atomics. Declined: a channel
per event, because a channel needs a reader task the ticker already is.

**Only `nvs serve` runs event entries**, as `rule:config/a-scheduled-run-is-a-root-isolate` says
for every entry. A job dead-lettered by a queue worker under `nvs run` is logged, as today, and
runs no `failed` entry. Declined: per-request events (an error page, an uncaught exception, a
request at its memory limit, request start or end), because an attacker sets their rate and they
grow with traffic; `Core\Log` and the `[metrics]`/`[trace]` exporters are their home. Declined:
an event for overload or a stuck core, because a run started then adds load at the worst moment.
Declined: a deploy or file-change event, which `reload` already is, and a "drain started" event,
which `stop` already is.

**The new rule is one fragment, `config/an-entry-runs-on-a-server-event`.** It holds the four
events, the args map, the floor, the `stop` bound and the Declined list above. The existing rules
change only where they say "cron" for every entry or "the entry's `name`" for the args.

**Chain position:** before `chain-ends-green`, after every security goal, because this is a feature
and the priority ordering puts security first. The user may move it with `looper goal move`.

**Lanes:** every stage runs in order. Stage 2 needs stage 1's `on` field, stage 3 shares
`crates/nvs-server/src/schedule.rs` and `crates/nvs-cli/src/serve.rs` with stage 2, and stage 4 tests
what 2 and 3 build.

## Stages

### 1. the on key is checked at boot

**Does:** add `on` to the `[[schedule]]` entry, check it at boot, and write the new rule.

- [ ] `Schedule` gets `pub on: Option<String>`, documented as the event an entry runs on
  (`config/an-entry-runs-on-a-server-event`, `crates/nvs-config/src/tree.rs:1264`).
- [ ] `validate` refuses, as `E0611` naming the entry and the line, both `cron` and `on`, neither,
  an unknown `on` value, `on` with `scope = "fleet"`, and `on` with `timezone`. `cron` stops being
  required on its own (`crates/nvs-config/src/schedule.rs:140`, `crates/nvs-config/src/schedule.rs:171`,
  `crates/nvs-config/src/schedule.rs:330`).
- [ ] An `Event` enum (`Start`, `Stop`, `Reload`, `Failed`) with one parser, which `validate` and
  stage 2's `arm` both call, so the four words have one home (`crates/nvs-config/src/schedule.rs:60`).
- [ ] The commented `[[schedule]]` block in `default.toml` shows an `on` entry, and the directive row
  stays `System` (`crates/nvs-config/src/default.toml:908`, `crates/nvs-config/src/directive.rs:317`).
- [ ] Test `an_event_entry_is_checked_at_boot`: each refusal above is `E0611` and names the line, and
  `on = "start"` with `scope = "host"` passes (`crates/nvs-config/tests/resolve.rs:1556`).
- [ ] Test `an_entry_names_exactly_one_of_cron_and_on` (`crates/nvs-config/tests/resolve.rs:1556`).
- [ ] The new fragment `docs/rules/config/an-entry-runs-on-a-server-event.md` from the standing
  decisions, its record `data/rules/config/an-entry-runs-on-a-server-event.json` with
  `crates/nvs-config/tests/resolve.rs` in `guardedBy`, its slug in the `rules` list of
  `data/rules/config.json`, and `bun nv rules --render` (`data/rules/config.json:1`).
- [ ] `rule:config/scheduled-work-is-a-config-block` shows `on` in its block and says an entry names
  `cron` or `on` (`docs/rules/config/scheduled-work-is-a-config-block.md:1`), and
  `rule:config/cron-is-five-fields-and-nothing-more` says `on` is not a cron shorthand
  (`docs/rules/config/cron-is-five-fields-and-nothing-more.md:1`).

### 2. start and stop entries run, and every run reads one map

**Does:** arm event entries beside cron entries, run `start` on the first pass and `stop` on the
drain, and give every run the args map.

- [ ] `Armed` carries a trigger, a cron expression or an event, and `arm` builds it with stage 1's
  one parser. An `on` entry needs no `cron` and has no `next` (`crates/nvs-server/src/schedule.rs:138`,
  `crates/nvs-server/src/schedule.rs:409`, `crates/nvs-server/src/schedule.rs:430`).
- [ ] `due` and `soonest` skip event entries, so a minute never runs one, and `carry_over` keeps an
  event entry's running count (`crates/nvs-server/src/schedule.rs:598`,
  `crates/nvs-server/src/schedule.rs:691`).
- [ ] `tick_on_this_core` runs each `start` entry once on its first pass, through the same `fire`
  and `overlap` path a cron entry takes (`crates/nvs-server/src/schedule.rs:641`,
  `crates/nvs-server/src/schedule.rs:881`).
- [ ] On the drain, the tick runs each `stop` entry and then waits for every fire it started, up to
  `[server] drain_timeout`, before it returns. The `keep_ticking` closure keeps its meaning, and the
  wait is the drain's own deadline (`crates/nvs-cli/src/serve.rs:1526`,
  `crates/nvs-config/src/server.rs:225`).
- [ ] `Fires::isolate` is told which trigger it fires for, and `Scheduled::isolate` hands the args map
  in place of the name string (`crates/nvs-server/src/schedule.rs:310`,
  `crates/nvs-cli/src/serve.rs:2108`, `crates/nvs-cli/src/serve.rs:2163`).
- [ ] Tests `a_start_entry_runs_once_on_the_first_pass`, `a_stop_entry_runs_on_the_drain_and_is_waited_for`
  and `a_minute_never_runs_an_event_entry` beside the ticker's own tests, with the injected clock
  (`crates/nvs-server/src/schedule.rs:1371`).
- [ ] Test `every_scheduled_run_reads_one_map_from_its_args` beside the serve fire tests
  (`crates/nvs-cli/src/serve.rs:3485`).
- [ ] `rule:config/a-scheduled-run-is-a-root-isolate` says the script reads the args map, and
  `rule:concurrency/one-process-serves-requests-schedules-and-jobs` says the drain runs `stop`
  entries and waits for them (`docs/rules/config/a-scheduled-run-is-a-root-isolate.md:17`,
  `docs/rules/concurrency/one-process-serves-requests-schedules-and-jobs.md:12`).

### 3. reload and failed entries run

**Does:** add the process-wide `Events`, feed it from the reload, the fire outcome and the
dead-letter move, and run `reload` and `failed` entries from the tick.

- [ ] `Events` in `crates/nvs-server/src/schedule.rs`, per the standing decision: a reload generation
  with the newest outcome and restart keys, and the failure tally (`crates/nvs-server/src/schedule.rs:553`).
- [ ] `serve.rs` makes one `Events` at boot and hands it to the ticker, the reload path and
  `arm_queue_workers` (`crates/nvs-cli/src/serve.rs:1445`, `crates/nvs-cli/src/serve.rs:1540`).
- [ ] The reload path writes each outcome into `Events` beside its `Core\Log` record: applied with
  its restart-pending key names, or refused. No value and no refusal text
  (`crates/nvs-cli/src/reload.rs:357`, `crates/nvs-cli/src/reload.rs:564`,
  `crates/nvs-cli/src/reload.rs:773`, `crates/nvs-cli/src/reload.rs:802`).
- [ ] `Scheduled::ran` and the not-run warnings count a failure for every entry that is not
  `on = "failed"` (`crates/nvs-cli/src/serve.rs:2129`, `crates/nvs-cli/src/serve.rs:2152`,
  `crates/nvs-cli/src/serve.rs:2186`).
- [ ] The dead-letter move counts one dead letter in `Events` once its statement succeeded, on every
  dialect (`crates/nvs-cli/src/worker.rs:1673`).
- [ ] Each pass of `tick_on_this_core` runs `reload` entries when the generation moved, and `failed`
  entries when the tally is not empty and a minute has passed since that entry's last run, taking
  the tally as it fires (`crates/nvs-server/src/schedule.rs:659`).
- [ ] Tests `a_reload_entry_runs_once_per_new_outcome`,
  `a_failed_entry_runs_at_most_once_a_minute_and_counts_every_failure` and
  `a_failed_entry_never_counts_its_own_failure`, with the injected clock
  (`crates/nvs-server/src/schedule.rs:1371`).

### 4. the server runs every event end to end, and the feature has its proofs

**Does:** prove each event against the built `nvs serve`, then write what a user reads.

**Split:** once the three tests pass, three writing subagents run side by side on disjoint files:
one writes example 02 and updates `about.md`, one writes attack 02, and one updates the two website
guides. Each is held to AGENTS.md § *Text an end user reads* and runs `bun nv proofs --comments`
on its own files. None of them builds.

- [ ] New test file `crates/nvs-cli/tests/schedule_events.rs`, built on the `Server` harness and the
  `scheduling` helper of `crates/nvs-cli/tests/live_config.rs:1200`. Each event script writes a file
  the test waits for, never a sleep (`crates/nvs-cli/tests/schedule_events.rs:1`).
- [ ] Test `a_start_and_a_stop_entry_run_under_nvs_serve`: the start file appears while the server
  serves, and the stop file exists when the stopped process has exited.
- [ ] Test `a_reload_entry_runs_after_a_saved_configuration`: a saved valid edit writes
  `outcome = applied`, and a saved broken edit writes `outcome = refused`.
- [ ] Test `a_failed_entry_runs_after_a_scheduled_script_throws`: a `* * * * *` entry that throws,
  and the failed script receives its name. This waits for the next whole minute, as
  `a_changed_schedule_roster_is_armed_from_the_next_tick` does (`crates/nvs-cli/tests/live_config.rs:1218`).
- [ ] `docs/examples/config/schedule/about.md` says an entry runs on a time or on one of four server
  events, and that `stop` does not run on a crash (`docs/examples/config/schedule/about.md:1`).
- [ ] Example `docs/examples/config/schedule/02-work-that-runs-on-a-server-event.nvs`: the `on`
  entries in a comment, and a script body that reads `Core\Script::args()` and prints which event it
  ran for (`docs/examples/config/schedule/02-work-that-runs-on-a-server-event.nvs:1`).
- [ ] Example 01's comment shows that a cron run reads the args map, if it says what the script
  receives (`docs/examples/config/schedule/01-work-that-runs-on-a-clock.nvs:1`).
- [ ] Attack `tests/hostile/config/schedule/02-flooding-the-failed-entry.nvs`: a request pushes many
  jobs that fail, and the `failed` entry still runs at most once a minute
  (`tests/hostile/config/schedule/02-flooding-the-failed-entry.nvs:1`).
- [ ] Each of the two website guides gets a short section with an `on = "start"` and an
  `on = "stop"` entry that posts to `hooks.example.com`
  (`website/src/content/docs/guides/simple-programs/scheduled-job.mdx:1`,
  `website/src/content/docs/guides/cookbook/schedule-job.mdx:1`).
- [ ] `bun nv proofs --id directive:schedule` prints `complete.` and `bun nv render --website
  --check` is green.

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Nothing is built yet. `[[schedule]]` runs cron entries only, armed in `nvs_server::arm`, ticked by
`tick_on_this_core` on one core, and fired through `Scheduled` in `crates/nvs-cli/src/serve.rs`. A
cron run receives its entry's name as a string.

### Next

Stage 1, the on key is checked at boot. Files: `crates/nvs-config/src/tree.rs`,
`crates/nvs-config/src/schedule.rs`, `crates/nvs-config/src/default.toml`,
`crates/nvs-config/tests/resolve.rs`, `data/rules/config.json` and three rule fragments.

- [ ] `crates/nvs-config/src/tree.rs:1264`: `Schedule` gets `on`.
- [ ] `crates/nvs-config/src/schedule.rs:140`: `validate` refuses the five `on` mistakes as `E0611`.
- [ ] `crates/nvs-config/src/schedule.rs:60`: the `Event` enum and its one parser.
- [ ] `crates/nvs-config/src/default.toml:908`: the commented block shows an `on` entry.
- [ ] `crates/nvs-config/tests/resolve.rs:1556`: tests `an_event_entry_is_checked_at_boot` and
  `an_entry_names_exactly_one_of_cron_and_on`.
- [ ] `data/rules/config.json:1`: the new rule `config/an-entry-runs-on-a-server-event`, its fragment
  and its record, then `bun nv rules --render`.
- [ ] `docs/rules/config/scheduled-work-is-a-config-block.md:1`: the block shows `on`
  (`docs/rules/config/cron-is-five-fields-and-nothing-more.md:1`).

No existing test is expected red before stage 2 changes the args a cron run receives. Stage 2 then
updates any test that reads a cron run's args as a string.

### Backlog
