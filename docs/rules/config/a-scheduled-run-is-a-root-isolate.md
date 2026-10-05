An isolate spends its parent's budget (`rule:security/isolate-budget-is-the-trees`). A scheduled run
has no parent, and that is not a new exception — it is the other existing shape: an inbound request
is the root isolate of a request tree, and a scheduled fire is a **second root**, built by the same
`Isolate` code path, on a task of its own so that a run taking an hour is not why the next minute's
entry is late. Everything downstream follows with nothing added:

- It runs under its script's own snapshot: the `[[app]]` blocks that match the script file, folded
  over the global tree (`rule:config/every-matching-app-block-applies-least-specific-first`), out of
  the publish serving when it fires. A script no block matches runs under the global tree, and one
  whose blocks do not fold is not run. This is the snapshot a request running that file would get.
- Its budget is that snapshot's `[limits]`, capped by `[limits.hard]`. A run that exceeds it is a
  `FATAL` handled by `rule:errors/escalation-ladder`'s ladder, which is why a runaway nightly job
  cannot take the serving cores with it.
- Its grants are that snapshot's `[capabilities]`. The script must still lie under the global
  `script.spawn` roots for the configuration to resolve (`E0611`), so a block can narrow what a
  fire may do and never makes a script schedulable.
- The script receives its entry's `name` through `Core\Script::args()`
  (`rule:core-classes/script-args`) and answers with a top-level `return`, exactly as any `spawn
  script` target does. There is no scheduler-specific accessor.
- `Core\Request`, `Core\Server` and `Core\Session` throw inside it
  (`rule:security/request-state-throws-in-an-isolate`) — there is no request.
- Its result is logged, not delivered: the `return` value goes into the run's log line, and an
  uncaught throw or a limit breach goes through the ladder with the entry's `name` in the record.
  Nothing is waiting for it.

**Only `nvs serve` runs schedules.** `nvs run`, `nvs check` and a bundled executable do not: a
schedule is a property of a running deployment, not of executing a file. The script is resolved per
fire, so an edited one is picked up at the next fire exactly as a request picks up an edited entry. A
queued job is the same root shape (`rule:concurrency/a-job-runs-as-a-root-isolate`).
