# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Both are on disk, and 0219 is the last record number this goal
takes. Stages 2 to 5 are complete.

**Stage 6.** The configuration check, its `config_check:` status line and its cost case are on disk
(`crate::control::check` in `crates/nvs-cli/src/control.rs`). `[server]` is a `Reload` block row with
`Boot` rows `listen`, `socket_mode` and `workers`. Every other `[server]` key reloads, and so do
`root`, `[[server.mount]]`, `[session]`, `[control] socket`, `[queue]`, `io.temp_root`,
`opcache.file_cache_dir` and now `http.client.tls` (`changed_anchors_judge_the_next_outbound_connection`).
The one row still `Boot` is `cache.shared`. The fragment `reloadability-is-its-own-field` names it in
its **What is on disk** paragraph. Remove it from there when it lands.

**The floor holds this goal red on a check it cannot fix.** Main's carried floor check
`nvs-config (the validate default)` names `the_validate_default_is_selected_by_the_run_mode`. This goal
renamed it to `validate_defaults_to_mtime_in_production_and_development` in `crates/nvs-config/tests/snapshot.rs`
(commit 8707a9148), by the standing decision that removes `never`: `validate`'s default is now `mtime`
in both modes, and the run mode selects `settle`'s instead. A test under the old name would claim
something false, and a side run may not edit main's `loop-goal.toml`. The user has to re-point that
floor check (in `docs/agent/loop-goal.toml` and `docs/agent/goals/dossier/115-core-http-response-and-1-more.toml`
on `main`) to the new name. Until then it stays red. The session that finds the rest of the goal green
writes `BLOCKED` on it.

My calls, not confirmed with the user: every published reload builds the `[http.client.tls]` client
again, rereading its anchor files, and `nvs_host::tls::install` swaps it in only when the anchors,
floor or key log differ. The pool key carries `nvs_host::tls::generation`, so a pooled socket from
before a swap never serves a later call. A block that does not build logs `configuration key not
applied`. It is named under `ignored:` only when the written block differs from the running one. A
reload does not print the `[capabilities.tls]` relaxation notes again. Earlier calls: a reload that
moves `file_cache_dir` builds the new cache before the publish, and a refused directory keeps the
running one and is named. A reload that moves `io.temp_root` runs no orphan sweep. The
`config_check:` line is always printed. A reload whose queue workers would move to storage behind the
queue's schema is refused whole. Older calls are in ADR 0218, ADR 0219 and `git log`.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main checkout.

## Next group

**Stage 6: the configuration applies itself** — one file set: `crates/nvs-cli/src/serve.rs`,
`crates/nvs-cli/src/control.rs`, `crates/nvs-config/src/directive.rs`,
`crates/nvs-config/tests/snapshot.rs`, `crates/nvs-config/tests/directives.rs`,
`crates/nvs-cli/tests/live_config.rs`.

- [ ] **`cache.shared` reloads**: row at `crates/nvs-config/src/directive.rs:185`. Requests already
      redial. The schedule ticker's fleet lease (`fleet_lease` at `crates/nvs-cli/src/serve.rs:1648`,
      opened once at boot) has to follow a changed store. `Process::trusting` at
      `crates/nvs-cli/src/control.rs:345` is the newest shape for a resource built before the publish
      and installed after it in `Process::published` at `crates/nvs-cli/src/control.rs:401`. Remove its
      `BOOT_CHANGES` row in `crates/nvs-config/tests/snapshot.rs`, add its `LIVE` line in
      `crates/nvs-config/tests/directives.rs`, and add a live case. `rule:config/reloadability-is-its-own-field`.
- [ ] **Only the three restart keys are `Boot`**: `only_listen_socket_mode_and_workers_need_a_restart`
      in `crates/nvs-config/tests/snapshot.rs:446`, once `cache.shared` is gone. This is the stage 6
      check. `rule:config/reloadability-is-its-own-field`.

## Backlog

- A service installed with a control socket stores its name for `ExecReload`
  (`control_socket` in `crates/nvs-cli/src/service.rs`), so after a move `systemctl reload`
  addresses the old name. The configuration check still applies the file.
- The three registry rows `server.listen`, `server.socket_mode` and `server.workers` are dossier
  features that owe their proofs. Stage 7's `dossier.py --gate` counts them.
- A reload that moves `file_cache_dir` to a refused directory and also moves the § 6 eviction keys
  publishes the new eviction values while the cache in force keeps the old policy
  (`Process::caching` in `crates/nvs-cli/src/control.rs`).
- `a_fleet_lease_is_renewed_while_its_run_is_in_flight` in `crates/nvs-server/src/schedule.rs` failed
  once under `verify.py`'s parallel load and passed alone and on the rerun.
- A pooled outbound socket from before a TLS client swap sits idle until
  `http.client.pool_idle_timeout` closes it (`pool_key` in `crates/nvs-stdlib/src/http/transport.rs`).
