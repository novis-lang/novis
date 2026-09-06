# ADR 0073 — Scheduled work is `nvs.toml` firing a `spawn script`, with a mandatory `scope`

- **Status:** Accepted
- **Date:** 2026-08-24
- **Scope:** the `[[schedule]]` array-of-tables in `nvs.toml`, its keys and their changeability class, the
  cron dialect accepted, what a scheduled run's budget and capabilities are, overlap handling, and the
  fleet-versus-host distinction. Not in scope: the shared store's own configuration, and `Core\Cache`'s
  member roster, both of which are [ADR 0059](0059-cross-request-state-is-explicit.md)'s and M8's.
- **Amends:** [0064](0064-configuration-file-format.md) — a new `[[schedule]]` array-of-tables.
  [0005](0005-config-changeability.md) — the block is `System`, and § 4 below says why every key is.
  [0006](0006-isolated-script-execution.md) — a scheduled run is a **root** isolate, the same shape an
  inbound request already is, so its budget question is answered by that ADR's existing rule rather than by
  a new exception. [docs/implementation-plan.md](../implementation-plan.md) — M6 gains the block's parsing
  and validation, M7 the ticker that fires it.
- **Amended by:** 0084

> **In short:** cron already exists, every deployment already runs it, and the only thing it does badly is
> that its job definition lives somewhere other than the application. So Novis takes the *declaration* and
> nothing else: a `[[schedule]]` entry in `nvs.toml` names a `cron` expression and a `script`, and fires it
> as an ordinary `spawn script` isolate ([ADR 0006](0006-isolated-script-execution.md)) — **no API surface
> at all**, no `Core\Schedule`, nothing a program can register at runtime. `nvs run` runs the identical
> file by hand, which is the whole debugging story. One key is **mandatory with no default**: **`scope`**,
> either `"fleet"` (once across the deployment, over the shared store) or `"host"` (once per host). Both
> are commonly correct and either default silently does the wrong thing in somebody's production, so the
> operator writes it down. `scope = "fleet"` with no shared store configured **refuses to boot**. There is
> no catch-up for a missed fire, and `"fleet"` is **at-most-once per interval, not exactly-once** — both
> stated here because both read as guarantees if they are not.

## Context

- Every non-trivial deployment has scheduled work: nightly reports, expiring sessions, retrying failed
  webhooks, rebuilding a search index. In PHP that means a system `crontab` line invoking
  `php /srv/www/bin/console app:thing`, and the job's existence, its schedule and its concurrency policy
  live in a file the application's repository does not contain, deployed by a mechanism the application's
  deployment does not run.
- The cost of that split is not theoretical: the schedule and the code drift, a job runs on all four
  application hosts because nobody remembered to guard it, and a `flock` guard is added per job by hand.
  The failure is silent — four copies of a nightly billing run look exactly like one until the invoices go
  out.
- **Novis already has every piece except the ticker.** `spawn script`
  ([ADR 0006](0006-isolated-script-execution.md)) runs a file in an isolate with its own arena, its own
  config overlay and capability narrowing. `nvs.toml` ([ADR 0064](0064-configuration-file-format.md)) is a
  root-owned file the operator already writes. `Core\Cache::shared()`
  ([ADR 0059](0059-cross-request-state-is-explicit.md)) is a coherent store across machines. What is
  missing is a clock and a lock, and both are small.
- **The API-surface question answers itself.** A runtime `Core\Schedule::register(...)` would be
  process-global mutable state ([ADR 0008](0008-static-and-global.md)) registered by whichever request
  happened to run the registering code first — a shape [ADR 0061](0061-compile-time-autoload-and-program-discovery.md)
  already rejected for autoloading, for the same reason. A schedule is deployment state, and
  [ADR 0064](0064-configuration-file-format.md) is where deployment state lives.
- **The `scope` question is the one the user settled explicitly.** Both answers are commonly correct: a
  cache warm should run on every host, a billing run must run once. Defaulting to `"host"` silently
  multiplies a fleet's side effects; defaulting to `"fleet"` silently disables per-host maintenance and
  makes the shared store a boot dependency for everyone. Requiring the key costs one line per entry and
  removes a class of production incident that is discovered by its consequences.

## Decision

### 1. The block

```toml
[[schedule]]                          # System, every key
name     = "nightly-report"           # required, unique — the log/metric label
cron     = "0 3 * * *"
script   = "jobs/report.nvs"
scope    = "fleet"                    # required, no default: "fleet" | "host"
timezone = "Europe/Vienna"            # default "UTC"
overlap  = "skip"                     # default "skip": "skip" | "queue" | "kill"
limits   = {memory = "512M", cpu_time = "120s"}   # optional sub-cap, narrowing only
grants   = {net.connect = ["reports.internal"]}   # optional, narrowing only

[[schedule]]
name  = "expire-sessions"
cron  = "*/15 * * * *"
script = "jobs/expire.nvs"
scope = "host"
```

An array-of-tables rather than one table with named sub-tables, because it is a repeated record with
several fields — [ADR 0064](0064-configuration-file-format.md) § 2's existing shape, the same one
`[[extension]]` uses. `name` is required and unique; a duplicate is a boot error naming both lines, exactly
as a duplicate key already is (§ 3 of that ADR).

`script` is resolved against the same `[capabilities] script.spawn` roots
[ADR 0006](0006-isolated-script-execution.md) already defines, canonicalised and prefix-checked. A path
outside them is a **boot** error, not a first-fire error: the set of files a deployment can execute is one
list, and a schedule is not a way around it.

### 2. Five-field cron, and nothing more

The accepted dialect is **five-field POSIX cron** — minute, hour, day-of-month, month, day-of-week — plus
the five named shorthands `@hourly`, `@daily`, `@weekly`, `@monthly`, `@yearly`. No seconds field, and none
of Quartz's `L`/`W`/`#`/`?` extensions.

The exclusions are the point, not an omission. A seconds field turns a scheduler into a timer, which is a
different feature with different accuracy expectations; and every dialect that adds one also adds the
`L`/`W`/`#` operators, which are the ones nobody can read six months later. Where a job genuinely needs
sub-minute work, it is a `@hourly` script holding a loop, which is visible and debuggable.

The expression is parsed and validated **at boot**, with the offending line named
([ADR 0064](0064-configuration-file-format.md) § 3's existing diagnostic shape). A typo'd schedule must not
be discovered by its silence.

### 3. `scope` decides who holds the lock

- **`scope = "host"`** — each host running `nvs serve` fires the entry on its own clock. No coordination,
  no shared store, no lock. Correct for anything whose effect is local: warming a per-core cache
  ([ADR 0059](0059-cross-request-state-is-explicit.md) § 1's local tier is per-core and per-host by
  construction), rotating a local file, sampling host state.
- **`scope = "fleet"`** — the entry fires once per interval across the whole deployment, guarded by a
  **lease** in the shared store keyed on `name` plus the fire's scheduled instant. A host that wins the
  lease runs the script; a host that does not, does not. The lease carries a TTL and is renewed while the
  run is in flight, so a host that dies mid-run releases it by expiry rather than blocking the next
  interval forever.

**`scope = "fleet"` with no shared store configured refuses to boot**, naming the entry. The alternative —
degrading to one run per host — is the exact failure this key exists to prevent, and a deployment that has
not configured a shared store has not decided anything about coordination yet.

**`"fleet"` is at-most-once per interval, not exactly-once**, and the ADR says so rather than leaving it to
be inferred. A network partition can leave an interval unrun; a lease expiring while a run is still alive
but unreachable can produce a second run. Exactly-once across machines needs a transaction the work itself
participates in, which is the application's job and not a scheduler's. A job that must not run twice makes
its own effect idempotent — which is the same advice a durable queue would give.

### 4. Every key is `System`

Nothing in this block is changeable from inside a request, and there is no `Core\Config::set` path to any
of it. A running request adding, removing or retiming a scheduled job would be process-global mutable state
under another name, and a request *narrowing* one would silently disable a job for everyone on that host —
so not even `RuntimeTighten` applies. This is the same class and the same reasoning
[ADR 0017](0017-hot-reload-without-restart.md) gives for `opcache.validate`.

### 5. A scheduled run is a root isolate, and spends a root's budget

[ADR 0006](0006-isolated-script-execution.md) fixes that an isolate spends its **parent's** budget. A
scheduled run has no parent, and this is not a new exception to that rule — it is that ADR's other existing
shape: *"an inbound HTTP request becomes the root isolate of a request tree."* A scheduled fire is a second
root, built by the same `Isolate` code path, and everything downstream follows with nothing added:

- Its budget is `[limits]`, capped by `[limits.hard]`, narrowed per entry by the optional `limits` table.
  A run that exceeds it is a `FATAL` handled by `rule:errors/escalation-ladder`'s ladder, which
  is why a runaway nightly job cannot take the serving cores with it.
- Its grants are the deployment's `[capabilities]`, narrowed per entry by the optional `grants` table.
  Narrowing only, as everywhere ([ADR 0005](0005-config-changeability.md)).
- The script receives its entry's `name` through `Core\Script::args()` and answers with a top-level
  `return`, exactly as any `spawn script` target does. There is no scheduler-specific accessor.
- `Core\Request`/`Core\Server`/`Core\Session` throw inside it, per
  [ADR 0012](0012-no-superglobals.md) — there is no request.
- Its result is logged, not delivered: a top-level `return` value is recorded in the run's log line, an
  uncaught throw or a limit breach goes through `rule:errors/escalation-ladder`'s ladder with
  the entry's `name` in the record. Nothing is waiting for it.

**Only `nvs serve` runs schedules.** `nvs run`, `nvs check` and a bundled
[ADR 0048](0048-portable-single-file-executables.md) executable do not: a schedule is a property of a
running deployment, not of executing a file. `nvs run jobs/report.nvs` runs the identical script by hand,
which is the entire debugging and backfill story and is why no `--run-now` flag is needed.

### 6. Overlap, and the missed fire

**`overlap`** decides what a fire does when the previous run of the same entry is still going:

- **`"skip"`** (default) — the fire is dropped and logged. The right default: a job that is still running
  when the next tick arrives is behind, and starting a second copy makes it further behind.
- **`"queue"` holds at most one pending run.** When a run finishes and one fire is pending, it starts
  immediately. A second overlap while one is already pending is dropped and logged — not held. An unbounded
  pending queue in front of a non-durable executor is precisely what
  [ADR 0072](0072-core-task-structured-concurrency.md) § 7 refuses to build, and it would be no better
  here.
- **`"kill"`** — the running isolate is cancelled at its next safepoint
  ([ADR 0006](0006-isolated-script-execution.md)), the scheduler waits for its teardown, then the new run
  starts. Cancellation runs no user code, per
  [ADR 0072](0072-core-task-structured-concurrency.md) § 5.

**A missed fire is never caught up.** A host that was down, a process that restarted, a clock that jumped
forward — in each case the missed interval is skipped and logged, and the next scheduled instant fires
normally. Catch-up needs a durable record of what has and has not run, which is a queue, which this is
deliberately not.

**Timezone and DST**, pinned because implementations differ and the difference is a production incident:
`timezone` defaults to `"UTC"` (there is no ambient timezone anywhere in Novis —
[ADR 0063](0063-core-api-conventions.md) § 4). A local-time schedule landing in a spring-forward **gap**
fires **once**, at the first valid instant after the gap. One landing in a fall-back **repeat** fires
**once**, on the first occurrence. Both rules are the ones that make "runs once a day" true, which is what
the operator wrote down.

## Consequences

**Positive**

- **The schedule ships with the application.** A job's existence, its cadence, its limits and its
  concurrency policy are in the same root-owned file as the rest of the deployment, reviewed in the same
  change, deployed by the same mechanism.
- **Fleet-once stops being per-deployment folklore.** The `flock`-and-hope pattern every PHP deployment
  reinvents becomes one required word, and the word is required precisely so nobody skips thinking about it.
- **Zero API surface** (priority 4). No `Core\Schedule`, no registration call, no attribute, no runtime
  state — the entire feature is a config block and a ticker.
- **The debugging story is one command.** `nvs run jobs/report.nvs` is the scheduled run, exactly, because
  a scheduled fire is nothing but that call. No dedicated "run this job now" subcommand and no divergence
  between the two paths to maintain.
- **Governance comes for free.** Limits, capabilities, the escalation ladder, the trace/metric plumbing and
  cancellation all apply because the run is an ordinary root isolate, not a special one.

**Negative**

- **`scope` is mandatory, and someone will find that annoying** the first time they add an entry. It is the
  whole point, and the boot error names both options with one sentence each.
- **`scope = "fleet"` makes the shared store a boot dependency.** A deployment that had not needed one now
  needs one to use fleet scheduling. Stated in the refusal rather than degraded around.
- **No catch-up, and no durability.** A missed nightly report stays missed until the next night or until
  someone runs it by hand. This is a real limitation relative to a durable scheduler, and it is the same
  boundary [ADR 0072](0072-core-task-structured-concurrency.md) § 6 draws for deferred work.
- **Five-field cron cannot express everything.** "The last weekday of the month" is a `@daily` script with
  a date check in it, which is more code and considerably more readable than `L-1W`.
- **A schedule is invisible to `nvs check`.** The scripts it names are ordinary files and are checked like
  any other, but nothing links an entry to a compiled program at compile time, so a renamed script is a
  boot error rather than a compile error. Boot-time path validation (§ 1) is what closes most of that gap.
- **One more block in a file that is growing.** `[[extension]]`, `[capabilities]`, `[limits]`, `[debug]`,
  `[log]`, `[db.<name>]` and now `[[schedule]]`. Each earns its place; the count is worth watching.

## Alternatives rejected

- **A runtime `Core\Schedule::register(cron, callable)`.** The framework-familiar shape (Laravel's
  `Kernel::schedule`). Rejected: process-global mutable state ([ADR 0008](0008-static-and-global.md))
  established by whichever request ran first, invisible to `nvs check`, and unreachable from a host that has
  not yet served a request — the same objections [ADR 0061](0061-compile-time-autoload-and-program-discovery.md)
  made against a runtime autoloader.
- **A `#[Schedule("0 3 * * *")]` attribute on a method**, discovered at compile time the way
  [ADR 0077](0077-compile-time-routing.md) discovers routes. Genuinely tempting, and rejected on lifetime:
  a schedule is deployment state, not source state. The same source tree is deployed to staging and
  production with different cadences, different limits and different scopes, and an attribute cannot carry
  that without a config file overriding it — at which point the config file is the schedule and the
  attribute is a second, weaker copy of it (R17).
- **Leaving it to system cron.** The status quo, zero Novis surface, and it works. Rejected on the split named
  in *Context*: the schedule lives outside the application, fleet-once has to be reinvented per job, and a
  cron-invoked `nvs run` gets no limits, no capability narrowing and no place to report a failure other than
  a mail spool.
- **Defaulting `scope` to `"host"`** (or to `"fleet"`). Rejected explicitly by the user, and correctly: both
  are commonly right, the wrong one is silent, and its consequence is either duplicated side effects or
  disabled maintenance.
- **Degrading `scope = "fleet"` to per-host when no shared store is configured**, with a warning. Rejected:
  a warning at boot is read once and then never again, and the failure it precedes is invoices going out
  four times.
- **Catch-up for missed fires.** Rejected: it needs a durable record of every interval's outcome, which is a
  queue with a scheduler bolted to it, and the applications that need one already run one.
- **Six-field cron with seconds, or Quartz's dialect.** Rejected in § 2: seconds make it a timer, and the
  dialects that offer them come with the unreadable operators attached.
- **An unbounded `overlap = "queue"`.** Rejected in § 6, for the same reason
  [ADR 0072](0072-core-task-structured-concurrency.md) § 7 refuses to queue deferred work: it hides an
  overload and then loses the work anyway.

## Revisiting

- **A `[[schedule]]` entry firing something other than a script** — an HTTP request to itself, a `Core`
  member — should stay rejected: `spawn script` is the one mechanism, and a second one is a second isolation
  path ([ADR 0006](0006-isolated-script-execution.md)).
- **Sub-minute cadence** if a real workload needs it. The answer is more likely a separate `[[timer]]`
  concept with its own accuracy contract than a seconds field bolted to cron.
- **Exactly-once fleet semantics** if an application appears that genuinely cannot be made idempotent. That
  needs the work itself to participate in a transaction, so it is a `Core\Db` design question, not a
  scheduler one.
- **The lease's store** is the shared tier today. If [ADR 0059](0059-cross-request-state-is-explicit.md)'s
  shared backend ever becomes plural, the lease needs to name which one, and this block gains a key.

## Verification

- **M6** (config): a `[[schedule]]` entry missing `scope` is a boot error naming both values; a malformed
  `cron` expression is a boot error naming the line; a duplicate `name` is a boot error naming both;
  a `script` outside `[capabilities] script.spawn`'s roots is a boot error, including one reaching outside
  through `..`; `scope = "fleet"` with no shared store configured refuses to boot; `Core\Config::set` on any
  scheduled key fails as a `System` set.
- **M7** (the ticker): an entry fires at its scheduled instant in its stated `timezone`; a schedule landing
  in a DST spring-forward gap fires exactly once, and one in a fall-back repeat fires exactly once —
  asserted against a fixed fake clock, not against wall-clock time.
- **M7:** `overlap = "skip"` drops an overlapping fire and logs it; `"queue"` holds exactly one pending run
  and drops the second; `"kill"` cancels the running isolate and starts the new run only after teardown
  completes.
- **M7:** a run's limit breach is a `FATAL` reported through
  `rule:errors/escalation-ladder`'s ladder with the entry's `name`, and the serving cores keep
  serving; a `Core\Request` call inside a scheduled script throws
  ([ADR 0012](0012-no-superglobals.md)); a scheduled run cannot widen a capability the deployment narrowed.
- **M7:** `nvs run` on a scheduled entry's script produces the identical observable behaviour to a fire —
  the same fixture run both ways.
- **M8** (shared store): two hosts racing the same fleet-scoped interval produce exactly one run; a host
  killed mid-run releases its lease by TTL expiry and the *next* interval fires normally, without the killed
  interval being retried.
