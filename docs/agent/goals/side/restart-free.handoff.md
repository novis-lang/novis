# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Both are on disk, and 0219 is the last record number this goal
takes. Stages 2 to 6 are complete.

**Stage 6 is done.** The only `Boot` rows are `server.listen`, `server.socket_mode` and
`server.workers` (`only_listen_socket_mode_and_workers_need_a_restart` in
`crates/nvs-config/tests/snapshot.rs`). `cache.shared` reloads: a request dials the store its own
snapshot names, and the schedule ticker in `crates/nvs-cli/src/serve.rs` opens its fleet lease again
when `[cache.shared]` moves. `nvs_server::Roster` carries the new lease to `tick_on_this_core`
(`a_changed_shared_store_takes_the_next_request_and_the_next_fleet_lease`). The fragment
`reloadability-is-its-own-field` no longer has a **What is on disk** paragraph, because all of it is
on disk.

**The floor holds this goal red on a check it cannot fix.** Main's carried floor check
`nvs-config (the validate default)` names `the_validate_default_is_selected_by_the_run_mode`. This goal
renamed it to `validate_defaults_to_mtime_in_production_and_development` in `crates/nvs-config/tests/snapshot.rs`
(commit 8707a9148), by the standing decision that removes `never`: `validate`'s default is now `mtime`
in both modes, and the run mode selects `settle`'s instead. A test under the old name would claim
something false, and a side run may not edit main's `loop-goal.toml`. The user has to re-point that
floor check (in `docs/agent/loop-goal.toml` and `docs/agent/goals/dossier/115-core-http-response-and-1-more.toml`
on `main`) to the new name. Until then it stays red. The session that finds the rest of the goal green
writes `BLOCKED` on it.

My calls, not confirmed with the user: the ticker opens a new lease when `[cache.shared]` moves and a
fleet entry exists, and when `[[schedule]]` changes while a fleet roster holds no lease (so a store
that did not answer at boot is tried again). It opens it on the ticker's own core, as the boot does.
A fire already running renews in the store it took its key from, and the old store is dropped when
the last such fire ends. A reload that moves the store names nothing as ignored: a store that will
not answer leaves fleet entries unarmed with a note, as at boot. Earlier calls are in ADR 0218, ADR
0219 and `git log`.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. Under full parallel load `verify.py` saw
`a_changed_queue_worker_count_starts_and_stops_workers_after_their_current_job` and nvs-host's
`the_watchdog_reports_a_wedged_worker_without_a_heartbeat` fail once and pass alone.
`tests/db/ca.crt` is a git-ignored fixture copied in from the main checkout.
`docs/examples/config/cache-shared/01-the-store-a-whole-fleet-shares.nvs` still misses the comment
bounds on its older lines 2, 15 and 22, which Stage 7's `--comments` check will name.

## Next group

**Stage 7: the template and the reference say it** — one file set: `crates/nvs-config/src/default.toml`,
`crates/nvs-config/tests/resolve.rs`, `docs/reference/tools/25-server.md`,
`docs/reference/tools/20-config.md`.

- [ ] **The three restart keys end `# restart required` in the template**: the `[server]` header at
      `crates/nvs-config/src/default.toml:428` still calls the whole block `Boot`-class. Mark
      `listen`, `socket_mode` and `workers`, and add
      `every_restart_key_is_marked_restart_required_in_the_template_and_no_other_is` beside
      `crates/nvs-config/tests/resolve.rs:223`, checking the template against `DIRECTIVES` in both
      directions. The `# default` / `# example` line endings have their own guard in `tools/`; keep
      both passing. `rule:config/reloadability-is-its-own-field`.
- [ ] **"What reaches a running server" reference section**: `docs/reference/tools/25-server.md:1`
      and `docs/reference/tools/20-config.md:54`. A code change always, a configuration change by
      itself, the three restart keys, how to deploy, and what no compiler makes consistent (the goal
      file's Stage 7 bullets). `rule:config/an-edit-reaches-the-next-request-without-a-restart`.

## Backlog

- Stage 7's dossier checks: `python tools/dossier.py --gate` and `--comments docs/examples tests/hostile benches/members` (goal file, Stage 7).
- The floor check `the_validate_default_is_selected_by_the_run_mode` needs the user to re-point it on `main` (see State).
