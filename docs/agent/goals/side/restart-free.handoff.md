# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Both are on disk, and 0219 is the last record number this goal
takes. Stages 2 to 5 are complete.

**Stage 6.** The configuration check is on disk (`crate::control::check` in
`crates/nvs-cli/src/control.rs`). `[server]` is a `Reload` block row with `Boot` rows `listen`,
`socket_mode` and `workers`, and every other `[server]` key reloads. `root` and `[[server.mount]]`
now reload too: `mounts::Rescan` (`crates/nvs-cli/src/serve/mounts.rs`) compares them in the
published snapshot with the tree it last expanded, and expands the published tree at once when they
differ. It renders a refusal against the source map `Process::published` records per snapshot
generation (`Process::sources_of`), reached through `crate::control::installed()`, which is no
longer Windows-only. Proved by `a_changed_mount_table_is_expanded_again`. Still `Boot`, each with its
own row and `BOOT_CHANGES` line: `http.client.tls`, `cache.shared`, `control.socket`,
`io.temp_root`, `opcache.file_cache_dir`, `session`, `queue.connection`, `queue.workers`. The
fragment `reloadability-is-its-own-field` names exactly these in its **What is on disk**
paragraph; shrink it as each lands.

My calls, not confirmed with the user: after boot, the check that a named file is one of the
table's entries is not asked again, and a reload that removes the last `[[server.mount]]` block
serves the named file alone (or keeps the rows, with a warning, where no file was named); a changed
root or block is expanded at once, without waiting for `settle`. Earlier calls: `[server]` is one
`Reload` block row with `Boot` rows beneath it; a `trusted_proxies` entry that names no network is
dropped on a reload without a note; the exporters keep the boot's waits for their own scrape and
push connections. Those before are in ADR 0218, ADR 0219 and the previous handoffs in `git log`.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 6: the configuration applies itself** — one file set: `crates/nvs-cli/src/control.rs`,
`crates/nvs-cli/src/serve.rs`, `crates/nvs-config/src/directive.rs`,
`crates/nvs-config/tests/snapshot.rs`, `crates/nvs-config/tests/directives.rs`,
`crates/nvs-cli/tests/live_config.rs`.

- [ ] **The resource keys' live cases** (ADR 0219 § 7): `a_changed_session_backend_applies_to_new_requests`,
      `a_changed_queue_worker_count_starts_and_stops_workers_after_their_current_job` and
      `a_changed_control_socket_moves_the_control_endpoint`, each after its subsystem rebuilds from
      the snapshot `Process::published` publishes at `crates/nvs-cli/src/control.rs:210`. Rows at
      `crates/nvs-config/src/directive.rs:205`, `crates/nvs-config/src/directive.rs:251` and
      `crates/nvs-config/src/directive.rs:267`.
- [ ] **`watching_the_configuration_costs_no_request_a_filesystem_call`** in
      `crates/nvs-cli/tests/live_config.rs`: the check runs on its own thread
      (`crates/nvs-cli/src/control.rs:409`), and the request path reads only `Current::load`.
      `rule:config/the-config-is-an-immutable-snapshot`.

## Backlog

- `only_listen_socket_mode_and_workers_need_a_restart` in `crates/nvs-config/tests/snapshot.rs`, once
  the last temporary `Boot` row is gone (stage 6 check).
- `http.client.tls`, `cache.shared`, `io.temp_root` and `opcache.file_cache_dir` still need a live
  case each, or a shared one (ADR 0219 § 7).
- The three registry rows `server.listen`, `server.socket_mode` and `server.workers` are dossier
  features that owe their proofs; stage 7's `dossier.py --gate` counts them.
