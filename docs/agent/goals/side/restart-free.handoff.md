# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4). It is on disk with the
five rules it modifies. Each fragment ends in a **What is on disk** paragraph, and the session that
lands the rest shrinks it.

**Stages 2, 3 and 4 are complete. Stage 5 is under way.** `[[app]] origin`, `[http.headers]`,
`[http.cors]` and `[opcache]` now reload. A reload's report names `app` when the `[[app]]` roster
moved (`Snapshot::roster`, compared as one leaf in `nvs_config::control::reload`).
`Compiler::reconfigure` moves `validate`, `revalidate_freq` and `settle` on each reload and wakes the
`watch` thread, so a shorter interval applies at once. `crates/nvs-cli/tests/live_config.rs` holds the
Stage 5 harness: `Server::start(case, config, files)`, `Server::reload(config)` (returns what `nvs ctl
reload` printed), `get_with`/`awaits_with`, `Answer::header`. A case that writes its own `[opcache]`
block replaces the harness's `QUICK_CHECKS`. Each of its four cases fails with its fix disabled.

Calls in ADR 0218 that are mine and not confirmed with the user: a mount re-expansion that meets a
match boot would refuse logs it and leaves it out (§ 9), keeping the replaced unit (§ 7, § 8), and
the `FLOOR` of 10ms between two watcher passes. Also mine: a row whose entry does not compile is
kept in the table, a `[server] root` that vanishes keeps the table as it stands, a reload that
removes `[[app]] origin` from a row whose unit calls `urlAbsolute` leaves that row out (404) rather
than refusing the reload, and a roster change is reported as the one key `app` rather than per block.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 5: every reloadable key really reloads** — one file set: `crates/nvs-server/src/admit.rs`,
`crates/nvs-cli/src/control.rs`, and `crates/nvs-cli/tests/live_config.rs`.

- [ ] **`limits.memory` moves the admission ceiling** (`rule:config/reloadability-is-its-own-field`).
      `Admission` takes `Ceiling::effective` once at boot and its doc calls that `Boot`-class, at
      `crates/nvs-server/src/admit.rs:147`. Make the ceiling reloadable (an atomic set from
      `Process::published`, `crates/nvs-cli/src/control.rs:138`), keep admitted requests counted, and
      rewrite that doc. Test `a_changed_memory_limit_moves_the_admission_ceiling` in
      `crates/nvs-cli/tests/live_config.rs`.
- [ ] **Queue jobs run under the snapshot in force when claimed**
      (`rule:config/reloadability-is-its-own-field`). Find where the worker reads `max_attempts` and
      `visibility`, from `crates/nvs-cli/src/worker.rs:1`. Test
      `a_queue_job_runs_under_the_configuration_in_force_when_it_is_claimed`.

## Backlog

- `[[schedule]]` armed from the next tick, and the metrics and trace exporters rebuilt — `docs/agent/goals/side/restart-free.md` § Stage 5.
- `crate::metrics::meter_this_core` reads the snapshot once per core at start (`crates/nvs-runtime/src/metrics.rs:816`), which the metrics/trace case will meet — same goal file, § Stage 5.
- The census, `every_directive_has_a_live_apply_proof_or_a_restart_proof` in `nvs-config` — same file, § Stage 5.
- Stage 6, the configuration applies itself, and `[[server.mount]]` / `[server] root` following a reload — same file, § Stage 6, and a second decision record.
- A module removed from the mount table keeps its path entry in the compiler, checked on every pass until the process ends — `crates/nvs-cli/src/script.rs`.
