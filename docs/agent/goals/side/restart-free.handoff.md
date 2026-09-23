# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4). It is on disk with the
five rules it modifies. Each fragment ends in a **What is on disk** paragraph, and the session that
lands the rest shrinks it.

**Stages 2, 3 and 4 are complete. Stage 5 is under way.** `[[app]] origin`, `[http.headers]`,
`[http.cors]`, `[opcache]` and `limits.memory` now reload. The admission ceiling is an atomic that
`Admission::resize` sets from `Process::published` after each publish (`crates/nvs-server/src/admit.rs`),
recomputed with `capacity_for` over the serving tree, so `[server] max_in_flight` still takes the
carried `Boot` value. `crates/nvs-cli/tests/live_config.rs` holds the Stage 5 harness:
`Server::start(case, config, files)`, `Server::reload(config)`, `Server::ctl(request)` (any `nvs ctl`
request, e.g. `status`), `get_with`/`awaits_with`, `Answer::header`. Each of its five cases fails with
its fix disabled.

**The configuration-apply decision record (Stages 5 and 6) is not written yet.** Its `changes.modifies`
must name `http-server/admission-is-arithmetic-not-a-number`, whose fragment already says a reload
recomputes the ceiling, and that rule's `because` must gain the record's number in the same commit.

Calls in ADR 0218 that are mine and not confirmed with the user: a mount re-expansion that meets a
match boot would refuse logs it and leaves it out (§ 9), keeping the replaced unit (§ 7, § 8), and
the `FLOOR` of 10ms between two watcher passes. Also mine: a row whose entry does not compile is
kept in the table, a `[server] root` that vanishes keeps the table as it stands, a reload that
removes `[[app]] origin` from a row whose unit calls `urlAbsolute` leaves that row out (404) rather
than refusing the reload, a roster change is reported as the one key `app` rather than per block,
and a lowered admission ceiling keeps every admitted request counted rather than cancelling any.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 5: every reloadable key really reloads** — one file set: `crates/nvs-cli/src/worker.rs`,
`crates/nvs-cli/src/control.rs`, and `crates/nvs-cli/tests/live_config.rs`.

- [ ] **Queue jobs run under the snapshot in force when claimed** (`rule:config/reloadability-is-its-own-field`).
      `worker::start` hands every worker the boot's snapshot once, at
      `crates/nvs-cli/src/worker.rs:236`, and `ctx.set_config` pins it at
      `crates/nvs-cli/src/worker.rs:248`. Take the snapshot from the published `Current` at each
      claim instead, so `[queue] max_attempts` and `visibility` read the tree in force. Test
      `a_queue_job_runs_under_the_configuration_in_force_when_it_is_claimed` in
      `crates/nvs-cli/tests/live_config.rs`.
- [ ] **`[[schedule]]` arms from the next tick** (`rule:config/reloadability-is-its-own-field`).
      Test `a_changed_schedule_roster_is_armed_from_the_next_tick` in
      `crates/nvs-cli/tests/live_config.rs:1`; locate the scheduler's roster read first
      (`python tools/peek.py --locate schedule`).

## Backlog

- `changed_metrics_and_trace_blocks_rebuild_their_exporters` — Stage 5, `live_config.rs`.
- The census test `every_directive_has_a_live_apply_proof_or_a_restart_proof` in `nvs-config` — Stage 5's second check.
- The configuration-apply decision record, with the `modifies` named in `## State` — goal `restart-free.md` standing decisions.
- `[server]` is still `Boot` whole in `crates/nvs-config/src/directive.rs:228`; the standing decision keeps only `listen`, `socket_mode` and `workers` restart-only.
