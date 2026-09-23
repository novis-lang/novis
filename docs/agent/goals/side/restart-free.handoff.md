# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Both are on disk, and 0219 is the last record number this goal
takes. Stages 2 to 5 are complete.

**Stage 6.** The configuration check is on disk (`crate::control::check` in
`crates/nvs-cli/src/control.rs`). It counts its passes and `stat` calls, and `nvs ctl status`
prints them as `config_check: P passes, S stats, N paths` (`nvs_server::control::Checked`).
`watching_the_configuration_costs_no_request_a_filesystem_call` sends many requests between two
readings and bounds both counts by the time alone. `[server]` is a `Reload` block row with `Boot`
rows `listen`, `socket_mode` and `workers`. Every other `[server]` key reloads, and so do `root`,
`[[server.mount]]`, `[session]`, `[control] socket`, all of `[queue]` and now `io.temp_root`
(`a_changed_temp_root_applies_to_the_next_temporary_directory`). Still `Boot`: `http.client.tls`,
`cache.shared`, `opcache.file_cache_dir`. The fragment `reloadability-is-its-own-field` names
exactly these in its **What is on disk** paragraph. Shrink it as each one lands.

My calls, not confirmed with the user: a reload that moves `io.temp_root` runs no orphan sweep. The
temporary directories made under the old root are deleted by path when their scripts end, and a
crashed process's leftovers there wait for `nvs tmp clean`, as the orphan-sweep rule already allows
(`rule:core-classes/temporary-dir-orphan-sweep` names exactly two places it runs). The `config_check:`
status line is always printed by `nvs serve`, also for a tree that reads no file (`0 paths`). A reload
whose queue workers would move to storage that is behind the queue's schema, or that cannot be
opened, is **refused whole**, as the boot is. Older calls are in ADR 0218, ADR 0219 and `git log`.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `nvs-host`'s `the_watchdog_reports_a_wedged_worker_without_a_heartbeat`
failed once beside the other test binaries and passed alone, a timing test under load. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 6: the configuration applies itself** — one file set: `crates/nvs-cli/src/script.rs`,
`crates/nvs-cli/src/control.rs`, `crates/nvs-config/src/directive.rs`,
`crates/nvs-config/tests/snapshot.rs`, `crates/nvs-config/tests/directives.rs`,
`crates/nvs-cli/tests/live_config.rs`.

- [ ] **`opcache.file_cache_dir` reloads** (ADR 0219 § 7): row at
      `crates/nvs-config/src/directive.rs:249`. The compiler's on-disk artifact cache has to follow
      the published directory: `Compiler::reconfigure` at `crates/nvs-cli/src/script.rs:605` is
      already called by `Process::published`. Remove its `BOOT_CHANGES` row in
      `crates/nvs-config/tests/snapshot.rs`, add its `LIVE` line in
      `crates/nvs-config/tests/directives.rs` and a live case. `rule:config/reloadability-is-its-own-field`.
- [ ] **`http.client.tls` reloads**: row at `crates/nvs-config/src/directive.rs:137`. It applies to
      the next outbound connection (ADR 0219 § 7); `crate::config::install_tls_client` at
      `crates/nvs-cli/src/serve.rs:196` installs it once at boot. `rule:config/reloadability-is-its-own-field`.

## Backlog

- `only_listen_socket_mode_and_workers_need_a_restart` in `crates/nvs-config/tests/snapshot.rs`, once
  the last temporary `Boot` row is gone (stage 6 check).
- `cache.shared` needs the schedule ticker's fleet lease (`fleet_lease` at
  `crates/nvs-cli/src/serve.rs:1648`, opened once at boot) to follow a changed store. Requests
  already redial.
- A service installed with a control socket stores its name for `ExecReload`
  (`control_socket` in `crates/nvs-cli/src/service.rs`), so after a move `systemctl reload`
  addresses the old name. The configuration check still applies the file.
- The three registry rows `server.listen`, `server.socket_mode` and `server.workers` are dossier
  features that owe their proofs. Stage 7's `dossier.py --gate` counts them.
