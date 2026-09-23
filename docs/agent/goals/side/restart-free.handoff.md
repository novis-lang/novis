# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Both are on disk, and 0219 is the last record number this goal
takes. Stages 2 to 5 are complete.

**Stage 6.** The configuration check is on disk (`crate::control::check` in
`crates/nvs-cli/src/control.rs`). `[server]` is a `Reload` block row with `Boot` rows `listen`,
`socket_mode` and `workers`; every other `[server]` key, `root`, `[[server.mount]]` and `[session]`
reload. `[control] socket` now reloads too: `Process::moving` creates the new endpoint before the
publish, `Process::open` starts its thread and retires the old one (`Door`, `retire`), and
`nvs_server::control::serve` takes a `retired` bit it reads between clients. A new endpoint that
cannot be created is logged (`configuration key not applied`) and the key is carried by
`Current::publish_keeping` and reported under `ignored`. Proved by
`a_changed_control_socket_moves_the_control_endpoint`. Still `Boot`, each with its own row and
`BOOT_CHANGES` line: `http.client.tls`, `cache.shared`, `io.temp_root`, `opcache.file_cache_dir`,
`queue.connection`, `queue.workers`. The fragment `reloadability-is-its-own-field` names exactly
these in its **What is on disk** paragraph; shrink it as each lands.

My calls, not confirmed with the user: a reload whose `[control] socket` names a network address is
refused whole with `E0629`, as the boot is; a control socket that could not be moved is listed by
`nvs ctl status` as a restart pending, because it is in the report's `ignored` list; after boot, the
check that a named file is one of the mount table's entries is not asked again; a reload that
removes the last `[[server.mount]]` block serves the named file alone; a changed root or block is
expanded at once, without waiting for `settle`. Earlier calls are in ADR 0218, ADR 0219 and the
previous handoffs in `git log`.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 6: the configuration applies itself** — one file set: `crates/nvs-cli/src/serve.rs`,
`crates/nvs-cli/src/worker.rs`, `crates/nvs-cli/src/control.rs`,
`crates/nvs-config/src/directive.rs`, `crates/nvs-config/tests/snapshot.rs`,
`crates/nvs-config/tests/directives.rs`, `crates/nvs-cli/tests/live_config.rs`.

- [ ] **`a_changed_queue_worker_count_starts_and_stops_workers_after_their_current_job`** (ADR 0219
      § 7). Rows at `crates/nvs-config/src/directive.rs:269` and
      `crates/nvs-config/src/directive.rs:270`. Workers are armed once on one core by
      `arm_queue_workers` at `crates/nvs-cli/src/serve.rs:1468`; a new count has to start tasks on
      that core or stop some after their current job, as the schedule ticker's roster does beside it.
      `queue.connection` is the same row pair's other half: each worker claims its next job on the
      new connection.
- [ ] **`watching_the_configuration_costs_no_request_a_filesystem_call`** in
      `crates/nvs-cli/tests/live_config.rs`: the check runs on its own thread
      (`crates/nvs-cli/src/control.rs:535`), and the request path reads only `Current::load`.
      `rule:config/the-config-is-an-immutable-snapshot`.

## Backlog

- `only_listen_socket_mode_and_workers_need_a_restart` in `crates/nvs-config/tests/snapshot.rs`, once
  the last temporary `Boot` row is gone (stage 6 check).
- `cache.shared` needs the schedule ticker's fleet lease (`fleet_lease` in
  `crates/nvs-cli/src/serve.rs`, opened once at boot) to follow a changed store; requests already
  redial.
- `http.client.tls`, `io.temp_root` and `opcache.file_cache_dir` still need a live case each, or a
  shared one (ADR 0219 § 7).
- A service installed with a control socket stores its name for `ExecReload`
  (`control_socket` in `crates/nvs-cli/src/service.rs`), so after a move `systemctl reload`
  addresses the old name; the configuration check still applies the file.
- The three registry rows `server.listen`, `server.socket_mode` and `server.workers` are dossier
  features that owe their proofs; stage 7's `dossier.py --gate` counts them.
