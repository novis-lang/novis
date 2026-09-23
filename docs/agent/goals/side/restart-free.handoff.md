# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4). It is on disk with the
five rules it modifies. Each fragment ends in a **What is on disk** paragraph, and the session that
lands the rest shrinks it.

**Stages 2, 3 and 4 are complete. Stage 5 is under way.** `[[app]] origin`, `[http.headers]`,
`[http.cors]`, `[opcache]`, `limits.memory`, the queue worker's snapshot and the `[[schedule]]`
roster now reload. The ticker is spawned on the ticking core even with no entry, takes a
`nvs_server::Rearm` (`crates/nvs-server/src/schedule.rs`, module doc § *A reload re-arms the roster
from the next pass*), waits at most `SCHEDULE_POLL` (1s, `crates/nvs-cli/src/serve.rs`) and re-arms
when the published `[[schedule]]` list differs; an entry kept by `name` keeps its run count, held
fire and next fire. `crates/nvs-cli/tests/live_config.rs` holds the Stage 5 harness:
`Server::start(case, config, files)`, `Server::start_after(.., first)`, `Server::reload(config)`,
`Server::ctl(request)`, `Server::said()`, `get_with`/`awaits_with`, `Answer::header`. Each of its
seven cases fails with its fix disabled.

**The configuration-apply decision record (Stages 5 and 6) is not written yet.** Its `changes.modifies`
must name `http-server/admission-is-arithmetic-not-a-number`, whose fragment already says a reload
recomputes the ceiling, and that rule's `because` must gain the record's number in the same commit.

Calls in ADR 0218 that are mine and not confirmed with the user: a mount re-expansion that meets a
match boot would refuse logs it and leaves it out (§ 9), keeping the replaced unit (§ 7, § 8), and
the `FLOOR` of 10ms between two watcher passes. Also mine: a row whose entry does not compile is
kept in the table, a `[server] root` that vanishes keeps the table as it stands, a reload that
removes `[[app]] origin` from a row whose unit calls `urlAbsolute` leaves that row out (404) rather
than refusing the reload, a roster change is reported as the one key `app` rather than per block,
a lowered admission ceiling keeps every admitted request counted rather than cancelling any, a
reload that removes the `[queue]` block leaves a running worker on the boot's `visibility`, the
1s `SCHEDULE_POLL` (a stop now ends the ticker within it), and a `fleet` entry a reload adds to a
boot that opened no lease is noted and not armed.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 5: every reloadable key really reloads** — one file set: `crates/nvs-cli/src/serve.rs`,
the exporter modules it builds from, and `crates/nvs-cli/tests/live_config.rs`.

- [ ] **Metrics and trace exporters rebuild on a reload** (`rule:config/reloadability-is-its-own-field`).
      Both collectors are built once from the boot snapshot at `crates/nvs-cli/src/serve.rs:551` and
      `crates/nvs-cli/src/serve.rs:563`. A new scrape socket is bound before the old one closes
      (goal § Stage 5). Test `changed_metrics_and_trace_blocks_rebuild_their_exporters` in
      `crates/nvs-cli/tests/live_config.rs`.
- [ ] **The census** `every_directive_has_a_live_apply_proof_or_a_restart_proof` in `nvs-config`
      (`docs/agent/goals/side/restart-free.toml:226`), once the exporter case lands; the registry is
      `crates/nvs-config/src/directive.rs`.

## Backlog

- The configuration-apply decision record, see `## State`.
- `nvs-server`'s `a_fleet_lease_is_renewed_while_its_run_is_in_flight`
  (`crates/nvs-server/src/schedule.rs`) failed once under the parallel `cargo test` and passed alone:
  its 40ms renewal inside a 130ms run is a timing bound that load breaks. It does not reach the ticker.
