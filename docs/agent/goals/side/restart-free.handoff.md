# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Both are on disk, and 0219 is the last record number this goal
takes. Stages 2 to 5 are complete.

**Stage 6.** The configuration check is on disk (`crate::control::check` in
`crates/nvs-cli/src/control.rs`). The `server` registry row is split: `server` is a `Reload` block
row, and `server.listen`, `server.socket_mode` and `server.workers` are its permanent `Boot` rows.
`dispatch`, `static`, `health_path` (the per-core table in `crates/nvs-cli/src/serve/mounts.rs`
rebuilds when the snapshot's switches differ), `trusted_proxies` (derived per snapshot in `Policy`,
`crates/nvs-server/src/serve.rs`) and `max_in_flight` now reload, proved by
`a_changed_server_block_reaches_the_next_request` and
`a_changed_in_flight_ceiling_moves_the_admission_ceiling`. Still `Boot`, each with its own row and
`BOOT_CHANGES` line: `server.root`, `server.mount`, the four waits, `server.drain_timeout`,
`server.connection`, `http.client.tls`, `cache.shared`, `control.socket`, `io.temp_root`,
`opcache.file_cache_dir`, `session`, `queue.connection`, `queue.workers`. The fragments
`reloadability-is-its-own-field` and `the-server-block-is-boot-class` carry a **What is on disk**
paragraph naming exactly these; shrink it as each lands.

My calls, not confirmed with the user: `[server]` is one `Reload` block row with `Boot` rows beneath it,
not one row per key, because every registry row is a dossier feature that owes its own proofs; a
`trusted_proxies` entry that names no network is dropped on a reload without a note (the boot still
reports it). The earlier calls are in ADR 0218, ADR 0219 and the previous handoffs in `git log`.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 6: the configuration applies itself** — one file set: `crates/nvs-server/src/serve.rs`,
`crates/nvs-cli/src/serve.rs`, `crates/nvs-config/src/directive.rs`,
`crates/nvs-config/tests/snapshot.rs`, `crates/nvs-config/tests/directives.rs`,
`crates/nvs-cli/tests/live_config.rs`.

- [ ] **The waits, `drain_timeout` and `[server.connection]` apply to the next connection** (ADR 0219
      § 7). Derive `Waits` and the connection bounds per snapshot beside the other policies in
      `Policy` at `crates/nvs-server/src/serve.rs:722`, and read them when a connection is accepted
      rather than from the value `crates/nvs-cli/src/serve.rs:209` resolves at boot. Flip the rows
      from `crates/nvs-config/src/directive.rs:242` to `Reload` (delete them: the `server` block row
      then governs them), drop their `BOOT_CHANGES` lines, and write
      `a_changed_server_timeout_applies_to_the_next_connection` in
      `crates/nvs-cli/tests/live_config.rs` as the census proof. Shrink the What-is-on-disk
      paragraphs of `rule:config/reloadability-is-its-own-field`,
      `rule:http-server/the-server-block-is-boot-class` and
      `rule:concurrency/connection-bounds-are-finite`.
- [ ] **`[server] root` and `[[server.mount]]` rebuild the mount table** (ADR 0219 § 7). The rescan
      reads them from the booted tree (`crates/nvs-cli/src/serve/mounts.rs:34`); read them from the
      published one and expand again when they changed. Rows at `crates/nvs-config/src/directive.rs:240`,
      and `rule:routing/an-origin-is-per-mount-and-checked-at-boot`.
- [ ] **The resource keys' live cases** (ADR 0219 § 7): `a_changed_session_backend_applies_to_new_requests`,
      `a_changed_queue_worker_count_starts_and_stops_workers_after_their_current_job` and
      `a_changed_control_socket_moves_the_control_endpoint`, each after its subsystem rebuilds from
      the snapshot `Process::published` publishes at `crates/nvs-cli/src/control.rs:195`. Rows at
      `crates/nvs-config/src/directive.rs:206`, `crates/nvs-config/src/directive.rs:260` and
      `crates/nvs-config/src/directive.rs:276`.

## Backlog

- `only_listen_socket_mode_and_workers_need_a_restart` in `crates/nvs-config/tests/snapshot.rs`, once
  the last temporary `Boot` row is gone (stage 6 check).
- `watching_the_configuration_costs_no_request_a_filesystem_call` in
  `crates/nvs-cli/tests/live_config.rs`: the check runs on its own thread
  (`crates/nvs-cli/src/control.rs:393`), and the request path reads only `Current::load`.
- `http.client.tls`, `cache.shared`, `io.temp_root` and `opcache.file_cache_dir` still need a live
  case each, or a shared one (ADR 0219 § 7).
- The three new registry rows `server.listen`, `server.socket_mode` and `server.workers` are dossier
  features that owe their proofs; stage 7's `dossier.py --gate` counts them.
