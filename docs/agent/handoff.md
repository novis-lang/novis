# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
stages 3 to 8 are complete, and stage 9's three checks are green. Nothing is blocked.

**A `[[schedule]]` entry's `limits` and `grants` narrow the run it fires.** `Armed` carries an
`nvs_runtime::host::Narrowing` built beside `overlap` when the entry is armed, and `fire` applies it
with `Isolate::narrowed_by`, so a sub-cap is the ticker's question exactly as `overlap` is. Nothing is
checked against the deployment first, because applying it *is* the check: a `limits` above the
deployment's ceiling leaves that ceiling standing, and a capability the deployment withheld is still
refused at the door. `rule:config/a-schedule-entry-narrows-only` is `shipped`, and now states the
decision this slice reached — a `grants` entry narrows by capability **name**, the scope written
beside it narrows nothing, and a second channel carrying scopes is refused rather than deferred.

**A fire's context carries no configuration, which is the module's one known gap**
(`crates/nvs-server/src/schedule.rs:78`). Read, not run: `fire` builds the run's root as
`Ctx::new(OutputSink::Sink)` (`crates/nvs-server/src/schedule.rs:759`),
`nvs_runtime::capability::granted` answers `false` for a context holding none
(`crates/nvs-runtime/src/capability.rs:133`), and `nvs_runtime::script::resolve` asks it for
`script.spawn` before compiling anything (`crates/nvs-runtime/src/script.rs:287`) — so under
`nvs serve` a fire is denied at the door, and the `limits` half of a sub-cap has no ceiling to
narrow. The grant half lands regardless: it subtracts from a list rather than setting a directive.

## Next group

**Stage 9: the schedule — the configuration a fire runs under** — one file set:
`crates/nvs-cli/src/serve.rs`, with `crates/nvs-server/src/schedule.rs` for the root it is put on.

- [ ] **A fire runs under the deployment's configuration** — `crates/nvs-cli/src/serve.rs:1218`'s
      `Scheduled::isolate` is handed the fire's own `&mut Ctx` and is the only side holding a
      snapshot, so it is the seam; `crates/nvs-cli/src/worker.rs:227` is the shape
      (`ctx.set_config(Arc::clone(&snapshot))`), and `crates/nvs-cli/src/serve.rs:930` is where
      `Scheduled` is built and would take one. The root it lands on is
      `crates/nvs-server/src/schedule.rs:759`, and the ticker's fake does the same thing in
      `crates/nvs-server/src/schedule.rs:1060` if a case there wants a second reading.
      `rule:config/a-scheduled-run-is-a-root-isolate` is what says a scheduled run gets the
      deployment's `[limits]` and its `[capabilities]`; that is the half that is missing. Test:
      `a_scheduled_fire_runs_under_the_deployments_configuration`, `-p nvs-cli`.
- [ ] **The snapshot a fire reads is the current one and not the boot's** — the ticker holds its
      `Rc<Scheduled>` for as long as the process runs (`crates/nvs-cli/src/serve.rs:930`), so a
      snapshot cloned into it at boot would outlive every reload, where a request resolves one per
      request. `rule:config/an-edit-reaches-the-next-request-without-a-restart` is the reading to
      apply to a fire, and `rule:config/a-reload-names-what-it-could-not-apply` is where a
      `[[schedule]]` key that cannot change says so. Decide it with the slice above.

## Backlog

- Stage 9 names no check for a fire's configuration — `docs/agent/loop-goal.toml:10322` covers the
  lease and the sub-cap only, which is why the gap above passed a green stage.
- A fleet entry whose fire is held by `queue` or started by `kill` runs under no lease at all —
  `crates/nvs-server/src/schedule.rs`'s tick walk answers § 6 before § 3 asks, so this host had
  already lost the interval. If it should take the key instead, that is § 3's reading and belongs
  in the rule.
- A fleet lease's key is `nvs:lease:` and the ticker's key with no application binding, so two
  deployments sharing one store share the lease for an entry they both name the same —
  `crates/nvs-stdlib/src/cache.rs`'s `LEASE_PREFIX` doc.
- Stage 8's "the check re-runs on reload" is vacuous while `[server]` is `Boot`-class: a reload
  cannot change a mount or its origin, which the rule fragment now says.
- The `unowned` gaps at `crates/nvs-server/src/route.rs:30`, `crates/nvs-server/src/bounds.rs:62`,
  `crates/nvs-types/src/response.rs:29` and `crates/nvs-stdlib/src/cli.rs:130` — goal
  `unowned-closures`.
- `Core\Metrics`'s three rows — goal `m8-stdlib-depth`.
