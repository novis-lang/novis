---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Scheduled work"
description: "A config block, not a runtime API: five cron fields, a mandatory scope, a lease across a fleet, and a missed fire that stays missed."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/config/application-blocks/
  label: "Application blocks"
next:
  link: /docs/rules/config/stores-and-caches/
  label: "Stores, sockets and the compiled-unit cache"
---

<p class="nv-section-lead">A config block, not a runtime API: five cron fields, a mandatory scope, a lease across a fleet, and a missed fire that stays missed.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">9</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">9</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">6</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#scheduled-work-is-a-config-block">Scheduled work is a <code>[[schedule]]</code> entry firing a <code>spawn script</code>, checked at boot, with no runtime API</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#cron-is-five-fields-and-nothing-more"><code>cron</code> is five-field POSIX plus five named shorthands, parsed at boot with the line named</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#scope-has-no-default"><code>scope</code> is mandatory with no default, and <code>fleet</code> with no shared store refuses to boot</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#every-schedule-key-is-system">Every <code>[[schedule]]</code> key is <code>System</code>, and not even <code>RuntimeTighten</code> reaches one</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-schedule-entry-narrows-only">An entry's <code>limits</code> and <code>grants</code> narrow the deployment's, and can never widen them</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#a-scheduled-run-is-a-root-isolate">A scheduled run is a root isolate that spends a root's budget, and only <code>nvs serve</code> fires one</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-fleet-entry-fires-at-most-once-under-a-lease">A <code>fleet</code> entry fires once per interval across the deployment under a lease in the shared store — at most once, never exactly once</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#overlap-is-skip-queue-or-kill"><code>overlap</code> is <code>skip</code> by default, <code>queue</code> holds exactly one pending run, and <code>kill</code> cancels before it starts</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#a-missed-fire-is-skipped-and-a-dst-edge-fires-once">A missed fire is never caught up, <code>timezone</code> defaults to UTC, and a DST gap or repeat fires exactly once</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="scheduled-work-is-a-config-block">

## Scheduled work is a `[[schedule]]` entry firing a `spawn script`, checked at boot, with no runtime API

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#scheduled-work-is-a-config-block"><code>config/scheduled-work-is-a-config-block</code></a>
</div>

Scheduled work is a `[[schedule]]` array-of-tables in `nvs.toml`, and nothing else: an entry names a
`cron` expression and a `script`, and the server fires that file as an ordinary `spawn script`
isolate ([`security/isolate-shares-nothing`](/docs/rules/security/isolates/#isolate-shares-nothing "Running another script is an in-process isolate that shares nothing with its parent but compiled code")). There is **no API surface** — no `Core\Schedule`,
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
target lives under ([`security/script-spawn-capability`](/docs/rules/security/closed-doors/#script-spawn-capability "Executing code is its own capability, and the entry path is canonicalised and then prefix-checked")), canonicalised and prefix-checked, and a
path outside them — including one reaching out through `..` — is a boot error, not a first-fire one:
the set of files a deployment can execute is one list. Nothing is deferred to the first fire, because
an entry that never fires looks exactly like one whose interval has not come round.

`nvs run jobs/report.nvs` runs the identical file by hand — the whole debugging and backfill story,
and why there is no `--run-now` flag. Durable, retried work is a different mechanism
([`concurrency/queued-work-is-not-scheduled-work`](/docs/rules/concurrency/running-a-job/#queued-work-is-not-scheduled-work "Queued work is durable, retried and declared by the application; scheduled work is a clock tick and none of those")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no crontab line and no scheduler API — a job, its cadence and its concurrency policy are an entry in <code>nvs.toml</code>, refused at boot when it cannot be armed</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/scheduled-work/#scope-has-no-default" title="scope is mandatory with no default, and fleet with no shared store refuses to boot"><code>config/scope-has-no-default</code></a> <a href="/docs/rules/config/scheduled-work/#cron-is-five-fields-and-nothing-more" title="cron is five-field POSIX plus five named shorthands, parsed at boot with the line named"><code>config/cron-is-five-fields-and-nothing-more</code></a> <a href="/docs/rules/config/scheduled-work/#a-scheduled-run-is-a-root-isolate" title="A scheduled run is a root isolate that spends a root's budget, and only nvs serve fires one"><code>config/a-scheduled-run-is-a-root-isolate</code></a> <a href="/docs/rules/security/isolates/#isolate-shares-nothing" title="Running another script is an in-process isolate that shares nothing with its parent but compiled code"><code>security/isolate-shares-nothing</code></a> <a href="/docs/rules/security/closed-doors/#script-spawn-capability" title="Executing code is its own capability, and the entry path is canonicalised and then prefix-checked"><code>security/script-spawn-capability</code></a> <a href="/docs/rules/concurrency/running-a-job/#queued-work-is-not-scheduled-work" title="Queued work is durable, retried and declared by the application; scheduled work is a clock tick and none of those"><code>concurrency/queued-work-is-not-scheduled-work</code></a> <a href="/docs/rules/config/the-file-and-the-tree/#lists-are-arrays-and-repeated-records-are-arrays-of-tables" title="A list is a TOML array, a repeated record is an array of tables, and a dotted key is table nesting"><code>config/lists-are-arrays-and-repeated-records-are-arrays-of-tables</code></a> <a href="/docs/rules/config/the-file-and-the-tree/#a-duplicate-key-is-an-error-and-so-is-an-unknown-one" title="A duplicate key is an error, and so is an unknown one — per file"><code>config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one</code></a> <a href="/docs/rules/config/includes-and-ownership/#a-value-array-replaces-and-a-table-appends" title="A value array is replaced wholesale by a later file; [[table]] entries accumulate across the tree"><code>config/a-value-array-replaces-and-a-table-appends</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0073.md">record 0073</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0064.md">record 0064</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/resolve.rs"><code>crates/nvs-config/tests/resolve.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="cron-is-five-fields-and-nothing-more">

## `cron` is five-field POSIX plus five named shorthands, parsed at boot with the line named

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#cron-is-five-fields-and-nothing-more"><code>config/cron-is-five-fields-and-nothing-more</code></a>
</div>

The accepted dialect is **five-field POSIX cron** — minute, hour, day-of-month, month, day-of-week —
plus exactly five named shorthands: `@hourly`, `@daily`, `@weekly`, `@monthly`, `@yearly`. Each
shorthand expands to its five-field form before anything reads it, so `@daily` and `0 0 * * *` are the
same schedule and nothing downstream can tell them apart. There is no seconds field and none of
Quartz's `L`, `W`, `#` or `?`.

The exclusions are the point. A seconds field turns a scheduler into a timer, which is a different
feature with a different accuracy contract, and every dialect that adds one also adds the operators
nobody can read six months later. Work that needs sub-minute cadence is an `@hourly` script holding a
loop, which is visible and debuggable; "the last weekday of the month" is a `@daily` script with a
date check in it.

The expression is parsed and validated **at boot**, with the offending line named — a typo'd schedule
must not be discovered by its silence. The one parse also answers the next fire: the scheduler never
re-reads the string, because two readers would be two dialects, and the two disagreeing is an entry
that booted and fires at the wrong minute, which nothing observes. When day-of-month and day-of-week
both narrow, POSIX's own rule holds and the entry fires on either.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A seconds field or Quartz's <code>L</code>/<code>W</code>/<code>#</code>/<code>?</code> is refused at boot rather than accepted by whichever cron the host happens to run</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/scheduled-work/#scheduled-work-is-a-config-block" title="Scheduled work is a [[schedule]] entry firing a spawn script, checked at boot, with no runtime API"><code>config/scheduled-work-is-a-config-block</code></a> <a href="/docs/rules/config/scheduled-work/#a-missed-fire-is-skipped-and-a-dst-edge-fires-once" title="A missed fire is never caught up, timezone defaults to UTC, and a DST gap or repeat fires exactly once"><code>config/a-missed-fire-is-skipped-and-a-dst-edge-fires-once</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0073.md">record 0073</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0064.md">record 0064</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/schedule.rs"><code>crates/nvs-config/tests/schedule.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/resolve.rs"><code>crates/nvs-config/tests/resolve.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="scope-has-no-default">

## `scope` is mandatory with no default, and `fleet` with no shared store refuses to boot

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#scope-has-no-default"><code>config/scope-has-no-default</code></a>
</div>

Every `[[schedule]]` entry writes `scope`, and there is no default. `"host"` fires the entry on every
host running `nvs serve`, each on its own clock, with no coordination and no lock — right for anything
whose effect is local: warming the per-core cache tier, rotating a local file, sampling host state.
`"fleet"` fires it once per interval across the whole deployment
([`config/a-fleet-entry-fires-at-most-once-under-a-lease`](/docs/rules/config/scheduled-work/#a-fleet-entry-fires-at-most-once-under-a-lease "A fleet entry fires once per interval across the deployment under a lease in the shared store — at most once, never exactly once")). An entry with neither, or with a third
word, refuses the boot naming both options with one sentence each.

The key is mandatory because both answers are commonly correct and either default silently does the
wrong thing in somebody's production. Defaulting to `"host"` multiplies a fleet's side effects — four
copies of a billing run look exactly like one until the invoices go out. Defaulting to `"fleet"`
silently disables per-host maintenance and makes the shared store a boot dependency for everyone.
One required word per entry removes a class of incident that is otherwise discovered by its
consequences.

**`scope = "fleet"` with no shared store configured refuses to boot**, naming the entry. The store is
`[cache.shared] url` ([`core-api/two-cache-tiers`](/docs/rules/core-api/lifetimes-and-absences/#two-cache-tiers "Cross-request state is reached through a member per tier, each with its own contract, never one API with a flag")'s coherent tier), and it is the only one. The
alternative — degrading to one run per host with a warning — is the exact failure the key exists to
prevent, and a warning at boot is read once and then never again.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The <code>flock</code>-and-hope guard a PHP deployment adds per cron job is one required word per entry, and forgetting it is a boot error rather than four copies of a nightly run</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/scheduled-work/#a-fleet-entry-fires-at-most-once-under-a-lease" title="A fleet entry fires once per interval across the deployment under a lease in the shared store — at most once, never exactly once"><code>config/a-fleet-entry-fires-at-most-once-under-a-lease</code></a> <a href="/docs/rules/config/scheduled-work/#scheduled-work-is-a-config-block" title="Scheduled work is a [[schedule]] entry firing a spawn script, checked at boot, with no runtime API"><code>config/scheduled-work-is-a-config-block</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#two-cache-tiers" title="Cross-request state is reached through a member per tier, each with its own contract, never one API with a flag"><code>core-api/two-cache-tiers</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0073.md">record 0073</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/resolve.rs"><code>crates/nvs-config/tests/resolve.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="every-schedule-key-is-system">

## Every `[[schedule]]` key is `System`, and not even `RuntimeTighten` reaches one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#every-schedule-key-is-system"><code>config/every-schedule-key-is-system</code></a>
</div>

Nothing in a `[[schedule]]` entry is changeable from inside a request, and there is no
`Core\Config::set` path to any of it. The whole block is `System`-class: read at boot, moved only by
a reload of the root-owned file, and refused as a `System` set from a program.

A running request adding, removing or retiming a scheduled job would be process-global mutable state
under another name ([`statements/static-is-a-member-modifier`](/docs/rules/statements/where-state-lives/#static-is-a-member-modifier "static marks a class member and names a class-relative type, and nothing else")), established by whichever request
happened to run the registering code first. And a request *narrowing* one — the direction every
other tightenable directive allows — would silently disable a job for everyone on that host, which is
why not even `RuntimeTighten` applies. This is the same class and the same reasoning that keeps
`opcache.validate` out of a request's reach.

`[queue]`, the durable job queue beside the schedule, is `System` throughout for the same reason: work
a request could redirect is work a request could redirect into a database it was never granted.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>No <code>ini_set</code> or <code>.user.ini</code> can add, remove or retime a job — a request cannot touch the schedule at all</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/scheduled-work/#scheduled-work-is-a-config-block" title="Scheduled work is a [[schedule]] entry firing a spawn script, checked at boot, with no runtime API"><code>config/scheduled-work-is-a-config-block</code></a> <a href="/docs/rules/statements/where-state-lives/#static-is-a-member-modifier" title="static marks a class member and names a class-relative type, and nothing else"><code>statements/static-is-a-member-modifier</code></a> <a href="/docs/rules/config/changeability-classes/#three-changeability-classes" title="nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes"><code>config/three-changeability-classes</code></a> <a href="/docs/rules/config/stores-and-caches/#opcache-revalidation-is-system-class" title="opcache.validate and its rate cap are System, and validate's startup default is chosen by the run mode"><code>config/opcache-revalidation-is-system-class</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0073.md">record 0073</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0017.md">record 0017</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/directives.rs"><code>crates/nvs-config/tests/directives.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-schedule-entry-narrows-only">

## An entry's `limits` and `grants` narrow the deployment's, and can never widen them

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#a-schedule-entry-narrows-only"><code>config/a-schedule-entry-narrows-only</code></a>
</div>

An entry's optional `limits` table is a sub-cap on the run's budget, and its optional `grants` table
narrows the run's capabilities. Both are **narrowing only**: an entry cannot raise `memory` above the
deployment's `[limits]`, and cannot grant a capability the deployment's `[capabilities]` withheld or
widen a scope it narrowed. A scheduled run cannot widen anything, for the same reason no program can
([`security/no-runtime-grant`](/docs/rules/security/scopes-and-denial/#no-runtime-grant "Two layers enforce a capability and only the static one grants; nothing anywhere widens")) — the root-owned file is the ceiling, and a block inside it is not a
second authority.

The ticker builds both when it arms the entry and applies them to each fire's isolate before its
first statement, which is the same narrowing a `spawn script` site writes and not a scheduler's own
mechanism. Nothing checks either table against the deployment first: a sub-cap wider than what is in
force leaves the inherited ceiling standing, and a name the deployment withheld is still refused at
the door, so the application is the check.

**`grants` narrows by capability name, and a scope written beside it narrows nothing.**
`grants = {net.connect = ["reports.internal"]}` holds the run to `net.connect` and leaves the hosts
the deployment named standing — the entry reaches no host `[capabilities]` withheld, and takes none
away from itself either. That is what a narrowing is everywhere, a spawn site's `grants:` included:
a list of names, asked beside the configuration rather than instead of it. A second channel carrying
scopes would make an entry a second place a capability's scope is resolved, which is the thing
[`security/capability-check-at-the-door`](/docs/rules/security/capabilities/#capability-check-at-the-door "The capability check lives inside the function that performs the effect, and that door is the only way out of the process") keeps to one.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/scheduled-work/#a-scheduled-run-is-a-root-isolate" title="A scheduled run is a root isolate that spends a root's budget, and only nvs serve fires one"><code>config/a-scheduled-run-is-a-root-isolate</code></a> <a href="/docs/rules/security/scopes-and-denial/#no-runtime-grant" title="Two layers enforce a capability and only the static one grants; nothing anywhere widens"><code>security/no-runtime-grant</code></a> <a href="/docs/rules/config/changeability-classes/#ceilings-are-their-own-directives" title="A ceiling is a System directive with the default's key name, and false removes it"><code>config/ceilings-are-their-own-directives</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0073.md">record 0073</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/schedule.rs"><code>crates/nvs-server/src/schedule.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-scheduled-run-is-a-root-isolate">

## A scheduled run is a root isolate that spends a root's budget, and only `nvs serve` fires one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-scheduled-run-is-a-root-isolate"><code>config/a-scheduled-run-is-a-root-isolate</code></a>
</div>

An isolate spends its parent's budget ([`security/isolate-budget-is-the-trees`](/docs/rules/security/isolates/#isolate-budget-is-the-trees "A request and everything it spawns share one budget, accounted at the tree's root")). A scheduled run
has no parent, and that is not a new exception — it is the other existing shape: an inbound request
is the root isolate of a request tree, and a scheduled fire is a **second root**, built by the same
`Isolate` code path, on a task of its own so that a run taking an hour is not why the next minute's
entry is late. Everything downstream follows with nothing added:

- Its budget is `[limits]`, capped by `[limits.hard]`. A run that exceeds it is a `FATAL` handled by
  [`errors/escalation-ladder`](/docs/rules/errors/the-escalation-ladder/#escalation-ladder "A failure escalates through four tiers, and no tier is retried")'s ladder, which is why a runaway nightly job cannot take the serving
  cores with it.
- Its grants are the deployment's `[capabilities]`.
- The script receives its entry's `name` through `Core\Script::args()`
  ([`core-classes/script-args`](/docs/rules/core-classes/processes-and-files/#script-args "Core\Script::args is the value the current isolate was spawned with, and null where there was none")) and answers with a top-level `return`, exactly as any `spawn
  script` target does. There is no scheduler-specific accessor.
- `Core\Request`, `Core\Server` and `Core\Session` throw inside it
  ([`security/request-state-throws-in-an-isolate`](/docs/rules/security/closed-doors/#request-state-throws-in-an-isolate "Request, server and session state throws where there is no inbound request, rather than answering empty")) — there is no request.
- Its result is logged, not delivered: the `return` value goes into the run's log line, and an
  uncaught throw or a limit breach goes through the ladder with the entry's `name` in the record.
  Nothing is waiting for it.

**Only `nvs serve` runs schedules.** `nvs run`, `nvs check` and a bundled executable do not: a
schedule is a property of a running deployment, not of executing a file. The script is resolved per
fire, so an edited one is picked up at the next fire exactly as a request picks up an edited entry. A
queued job is the same root shape ([`concurrency/a-job-runs-as-a-root-isolate`](/docs/rules/concurrency/running-a-job/#a-job-runs-as-a-root-isolate "A job runs as a root isolate, and there is no second execution path")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A cron-invoked <code>php</code> process gets no limits, no capability narrowing and no place to report but a mail spool; a scheduled run gets the deployment's <code>[limits]</code>, its <code>[capabilities]</code> and the escalation ladder because it is an ordinary isolate</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/isolates/#isolate-budget-is-the-trees" title="A request and everything it spawns share one budget, accounted at the tree's root"><code>security/isolate-budget-is-the-trees</code></a> <a href="/docs/rules/security/closed-doors/#request-state-throws-in-an-isolate" title="Request, server and session state throws where there is no inbound request, rather than answering empty"><code>security/request-state-throws-in-an-isolate</code></a> <a href="/docs/rules/statements/where-state-lives/#no-host-populated-variables" title="No variable is ever populated by the host; every superglobal is a Core class member"><code>statements/no-host-populated-variables</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a> <a href="/docs/rules/core-classes/processes-and-files/#script-args" title="Core\Script::args is the value the current isolate was spawned with, and null where there was none"><code>core-classes/script-args</code></a> <a href="/docs/rules/concurrency/running-a-job/#a-job-runs-as-a-root-isolate" title="A job runs as a root isolate, and there is no second execution path"><code>concurrency/a-job-runs-as-a-root-isolate</code></a> <a href="/docs/rules/config/scheduled-work/#a-schedule-entry-narrows-only" title="An entry's limits and grants narrow the deployment's, and can never widen them"><code>config/a-schedule-entry-narrows-only</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0073.md">record 0073</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/schedule.rs"><code>crates/nvs-server/src/schedule.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-fleet-entry-fires-at-most-once-under-a-lease">

## A `fleet` entry fires once per interval across the deployment under a lease in the shared store — at most once, never exactly once

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#a-fleet-entry-fires-at-most-once-under-a-lease"><code>config/a-fleet-entry-fires-at-most-once-under-a-lease</code></a>
</div>

A `scope = "fleet"` entry fires once per interval across the deployment, guarded by a **lease** in the
shared store keyed on the entry's `name` plus the fire's *scheduled* instant — not the instant it was
noticed, so two hosts whose clocks differ by a second still ask for the same key. A host that wins the
lease runs the script; a host that does not, does not. The lease carries a TTL and is renewed while
the run is in flight, so a host that dies mid-run releases it by expiry rather than blocking the next
interval forever.

**`"fleet"` is at-most-once per interval, not exactly-once.** A network partition can leave an
interval unrun; a lease expiring under a run that is alive but unreachable can produce a second run.
Exactly-once across machines needs a transaction the work itself participates in, which is the
application's job and not a scheduler's: a job that must not run twice makes its own effect
idempotent.

The ticker holds no store: it asks one question — take this key for this long, yes or no — through a
`Leases` parameter only `nvs serve` can supply, because the crate the ticker lives in names no
standard library. That binary supplies one whenever the tree names a `[cache.shared]` store its boot
can reach, over a connection of its own to that tier and a set-if-absent no program is given
([`concurrency/cross-request-state-is-explicit`](/docs/rules/concurrency/deferred-and-cross-request-state/#cross-request-state-is-explicit "A value outlives the request that made it only by being put into a named store")).

A tree with no shared store, and one whose store will not answer the boot, leave every `fleet` entry
**unarmed** and named in a boot note. Firing it on each host's own clock would be the precise failure
the scope exists to prevent, so the safe half is to run none of them and say so.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/scheduled-work/#scope-has-no-default" title="scope is mandatory with no default, and fleet with no shared store refuses to boot"><code>config/scope-has-no-default</code></a> <a href="/docs/rules/config/scheduled-work/#a-missed-fire-is-skipped-and-a-dst-edge-fires-once" title="A missed fire is never caught up, timezone defaults to UTC, and a DST gap or repeat fires exactly once"><code>config/a-missed-fire-is-skipped-and-a-dst-edge-fires-once</code></a> <a href="/docs/rules/concurrency/deferred-and-cross-request-state/#a-cached-value-is-copied-across-the-boundary" title="A value is copied into the cache and copied back out, by the same graph copy the isolate boundary uses"><code>concurrency/a-cached-value-is-copied-across-the-boundary</code></a> <a href="/docs/rules/concurrency/deferred-and-cross-request-state/#cross-request-state-is-explicit" title="A value outlives the request that made it only by being put into a named store"><code>concurrency/cross-request-state-is-explicit</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0073.md">record 0073</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0059.md">record 0059</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/schedule.rs"><code>crates/nvs-server/src/schedule.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="overlap-is-skip-queue-or-kill">

## `overlap` is `skip` by default, `queue` holds exactly one pending run, and `kill` cancels before it starts

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#overlap-is-skip-queue-or-kill"><code>config/overlap-is-skip-queue-or-kill</code></a>
</div>

`overlap` decides what a fire does when the previous run of the **same entry** is still going, and
the count it is asked of is that entry's own running fires — never a process-wide tally, or a nightly
report could suppress an hourly one.

- **`"skip"`** (the default) drops the fire and logs it. A job still running when the next tick
  arrives is behind, and starting a second copy makes it further behind.
- **`"queue"` holds at most one pending run.** When the run finishes and a fire is pending, it starts
  immediately — the wait is on the run *ending*, not on the next minute. A second overlap while one is
  already pending is dropped and logged, not held: an unbounded pending queue in front of a
  non-durable executor is what [`concurrency/deferred-is-bounded-by-two-directives`](/docs/rules/concurrency/deferred-and-cross-request-state/#deferred-is-bounded-by-two-directives "[deferred] carries two bounds: a System max_concurrent and a Runtime deadline") refuses to
  build, and it would be no better here.
- **`"kill"`** cancels the running isolate at its next safepoint, waits for its teardown, then starts
  the new run. Cancellation runs no user code ([`concurrency/cancellation-runs-no-user-code`](/docs/rules/concurrency/tasks/#cancellation-runs-no-user-code "A cancelled task runs no catch, no cleanup and no handler, and cancellation is not a Throwable")).

A word that is none of the three refuses the boot; the key has a default, and a mode the operator
asked for and will not get is refused rather than corrected. A dropped or held fire rearms like any
other, so a job that runs long falls behind by intervals rather than by copies.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/scheduled-work/#a-missed-fire-is-skipped-and-a-dst-edge-fires-once" title="A missed fire is never caught up, timezone defaults to UTC, and a DST gap or repeat fires exactly once"><code>config/a-missed-fire-is-skipped-and-a-dst-edge-fires-once</code></a> <a href="/docs/rules/concurrency/deferred-and-cross-request-state/#deferred-is-bounded-by-two-directives" title="[deferred] carries two bounds: a System max_concurrent and a Runtime deadline"><code>concurrency/deferred-is-bounded-by-two-directives</code></a> <a href="/docs/rules/concurrency/tasks/#cancellation-runs-no-user-code" title="A cancelled task runs no catch, no cleanup and no handler, and cancellation is not a Throwable"><code>concurrency/cancellation-runs-no-user-code</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0073.md">record 0073</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/schedule.rs"><code>crates/nvs-server/src/schedule.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/resolve.rs"><code>crates/nvs-config/tests/resolve.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-missed-fire-is-skipped-and-a-dst-edge-fires-once">

## A missed fire is never caught up, `timezone` defaults to UTC, and a DST gap or repeat fires exactly once

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-missed-fire-is-skipped-and-a-dst-edge-fires-once"><code>config/a-missed-fire-is-skipped-and-a-dst-edge-fires-once</code></a>
</div>

Every fire is computed from **now**, never counted from the entry's last fire, and that one choice is
the whole implementation of three rules.

**A missed fire is never caught up.** A host that was down, a process that restarted, a machine that
was suspended, a clock that jumped forward — in each case the missed interval is skipped and logged,
and the next scheduled instant fires normally. Catch-up needs a durable record of what has and has not
run, which is a queue, which this is deliberately not
([`concurrency/after-response-outlives-the-connection`](/docs/rules/concurrency/deferred-and-cross-request-state/#after-response-outlives-the-connection "Core\Task::afterResponse runs after the request's own frame returns, still charged to the request tree") draws the same boundary for deferred work).
A wall-clock step forwards costs the intervals it stepped over rather than firing them in a burst.

**`timezone` defaults to `"UTC"`.** There is no ambient timezone anywhere in Novis, so an absent key
is the documented default rather than the host's setting, and a name no IANA database knows refuses
the boot.

**A DST edge fires exactly once.** A local-time schedule landing in a spring-forward *gap* fires once,
at the first valid instant after the gap. One landing in a fall-back *repeat* fires once, on the first
occurrence. Both are decided in the single place a civil minute becomes an instant, and both are what
make "runs once a day" true, which is what the operator wrote down.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no ambient timezone to fall back on and no anacron-style replay — the interval a host slept through is logged as skipped, not run late</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/scheduled-work/#overlap-is-skip-queue-or-kill" title="overlap is skip by default, queue holds exactly one pending run, and kill cancels before it starts"><code>config/overlap-is-skip-queue-or-kill</code></a> <a href="/docs/rules/config/scheduled-work/#cron-is-five-fields-and-nothing-more" title="cron is five-field POSIX plus five named shorthands, parsed at boot with the line named"><code>config/cron-is-five-fields-and-nothing-more</code></a> <a href="/docs/rules/concurrency/deferred-and-cross-request-state/#after-response-outlives-the-connection" title="Core\Task::afterResponse runs after the request's own frame returns, still charged to the request tree"><code>concurrency/after-response-outlives-the-connection</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0073.md">record 0073</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/schedule.rs"><code>crates/nvs-config/tests/schedule.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/schedule.rs"><code>crates/nvs-server/src/schedule.rs</code></a></dd></div></dl>

</div>
