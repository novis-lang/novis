# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4). It is on disk with the
five rules it modifies. Each fragment ends in a **What is on disk** paragraph, and the session that
lands the rest shrinks it.

**Stages 2, 3 and 4 are complete. Stage 5 is under way.** `[[app]] origin`, `[http.headers]`,
`[http.cors]`, `[opcache]`, `limits.memory` and the queue worker's snapshot now reload. A queue
worker holds the published `nvs_config::Current` (`crates/nvs-cli/src/worker.rs`, `start`), sets
the job's context from it right after each claim, and reads `[queue] visibility` from it at the top
of each turn (`worker::window`); `nvs run` hands it a holder nothing publishes into.
`crates/nvs-cli/tests/live_config.rs` holds the Stage 5 harness: `Server::start(case, config, files)`,
`Server::start_after(.., first)` (runs `nvs <first>` before the boot, e.g. `queue migrate`),
`Server::reload(config)`, `Server::ctl(request)`, `Server::said()` (standard error so far),
`get_with`/`awaits_with`, `Answer::header`. Each of its six cases fails with its fix disabled.

**The configuration-apply decision record (Stages 5 and 6) is not written yet.** Its `changes.modifies`
must name `http-server/admission-is-arithmetic-not-a-number`, whose fragment already says a reload
recomputes the ceiling, and that rule's `because` must gain the record's number in the same commit.

Calls in ADR 0218 that are mine and not confirmed with the user: a mount re-expansion that meets a
match boot would refuse logs it and leaves it out (§ 9), keeping the replaced unit (§ 7, § 8), and
the `FLOOR` of 10ms between two watcher passes. Also mine: a row whose entry does not compile is
kept in the table, a `[server] root` that vanishes keeps the table as it stands, a reload that
removes `[[app]] origin` from a row whose unit calls `urlAbsolute` leaves that row out (404) rather
than refusing the reload, a roster change is reported as the one key `app` rather than per block,
a lowered admission ceiling keeps every admitted request counted rather than cancelling any, and a
reload that removes the `[queue]` block leaves a running worker on the boot's `visibility`.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 5: every reloadable key really reloads** — one file set: `crates/nvs-cli/src/serve.rs`,
`crates/nvs-server/src/schedule.rs`, and `crates/nvs-cli/tests/live_config.rs`.

- [ ] **`[[schedule]]` arms from the next tick** (`rule:config/reloadability-is-its-own-field`).
      The roster is armed once from the boot snapshot at `crates/nvs-cli/src/serve.rs:1137`, and the
      ticker is spawned only when that roster is not empty (`crates/nvs-cli/src/serve.rs:1149`), so
      a reload that adds the first entry has no ticker to reach. `nvs_server::tick_on_this_core`
      (`crates/nvs-server/src/schedule.rs:529`) loops over a fixed `&mut [Armed]` and returns when
      the roster has no fire left. Re-arm with `nvs_server::arm` (`crates/nvs-server/src/schedule.rs:371`)
      when `current.load().config.schedule` differs from what was armed; a firing in flight runs to
      completion. Decide what a `fleet` entry added by a reload does when the boot opened no lease
      (`crates/nvs-cli/src/serve.rs:1124`): arming it without one breaks § 3, so the safe answer is
      the existing note and skip. Rewrite the `Scheduled` doc (`crates/nvs-cli/src/serve.rs:1640`),
      which still says nothing re-arms. Test
      `a_changed_schedule_roster_is_armed_from_the_next_tick` in `crates/nvs-cli/tests/live_config.rs`:
      a `* * * * *` entry fires within a minute, so the case waits up to one more (live_edit's
      `a_queue_job_and_a_scheduled_fire_run_the_edited_code` is the pattern).
- [ ] **Metrics and trace exporters rebuild on a reload** (`rule:config/reloadability-is-its-own-field`).
      Both collectors are built once from the boot snapshot at `crates/nvs-cli/src/serve.rs:551` and
      `crates/nvs-cli/src/serve.rs:563`. Test `changed_metrics_and_trace_blocks_rebuild_their_exporters`.

## Backlog

- The census test `every_directive_has_a_live_apply_proof_or_a_restart_proof` in `nvs-config`
  (restart-free.toml:226), after the last live case lands.
- The configuration-apply decision record, see `## State`.
