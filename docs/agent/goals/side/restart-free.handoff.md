# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Both are on disk, and 0219 is the last record number this goal
takes. Each 0218 fragment ends in a **What is on disk** paragraph, and the session that lands the
rest shrinks it.

**Stages 2 to 5 are complete.** The census
`every_directive_has_a_live_apply_proof_or_a_restart_proof` in
`crates/nvs-config/tests/directives.rs` maps every registry row to exactly one proof: a `Reload`
row names a case in `crates/nvs-cli/tests/live_config.rs`, a `Boot` row names
`every_boot_row_a_reload_changes_is_named_and_keeps_its_running_value` in
`crates/nvs-config/tests/snapshot.rs`.

**Stage 6, the configuration check, is on disk.** `crate::control::check` in
`crates/nvs-cli/src/control.rs` stats every path of the serving tree (`Snapshot::files` and the new
`Snapshot::probed`) every two seconds and publishes a change that held for one check, under the
reload lock. It logs a refusal once per rendered diagnostic, and it logs a pending restart key once
per written value. `nvs ctl status` lists the pending keys (`Controlled::pending` in
`crates/nvs-server/src/control.rs`). Three of the eight `live_config` names in the stage's check
pass: the saved file, the half-written file and the pending restart key. The `Boot` set is still
the wide one, so the pending case shows the key `server` with the whole block as its value.

Calls that are mine and not confirmed with the user. In ADR 0219: the check is a fixed two seconds
(§ 4), a moved stamp must hold for one more check (§ 4), a resource that cannot be built keeps its
running value (§ 7), and a changed session backend does not carry sessions over (§ 7). From this
session: a tree equal to the serving one (table, roster, blocks, files, probed, secrets, origin
paths) publishes nothing; a refusal is deduplicated by its rendered text; a server on the shipped
defaults has nothing to check (a `nvs.toml` created later is not picked up); secret files a tree
reads are not stat'ed. The earlier calls from ADR 0218 and Stage 5 are listed in the two records
and in `git log`.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 6: the configuration applies itself** — one file set: `crates/nvs-config/src/directive.rs`,
`crates/nvs-config/tests/snapshot.rs`, `crates/nvs-config/tests/directives.rs`,
`crates/nvs-cli/tests/live_config.rs`.

- [ ] **`Boot` shrinks to three keys** (`rule:config/reloadability-is-its-own-field`, ADR 0219 § 6
      and § 7). Split the `server` row at `crates/nvs-config/src/directive.rs:228` into one row per
      key. `server.listen`, `server.socket_mode` and `server.workers` stay `Boot`; every other row
      that leaves `Boot` moves out of `BOOT_CHANGES` at `crates/nvs-config/tests/snapshot.rs:457`
      and names a live case in the census. Name `only_listen_socket_mode_and_workers_need_a_restart`
      in `crates/nvs-config/tests/snapshot.rs`. The pending case in
      `crates/nvs-cli/tests/live_config.rs` already accepts `server.workers` as the key. Rewrite the
      fragments that still say `Boot` as each key lands: `reloadability-is-its-own-field`,
      `http-server/the-server-block-is-boot-class` (title and body; the id stays),
      `config/a-startup-default-is-never-flipped` (the `dispatch` and `static` rows),
      `http-server/a-request-resolves-in-five-steps`,
      `http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot`,
      `concurrency/connection-bounds-are-finite`, `routing/an-origin-is-per-mount-and-checked-at-boot`
      and `config/one-local-control-socket` (the endpoint moves on a reload).
- [ ] **The resource keys' live cases** (ADR 0219 § 7), in `crates/nvs-cli/tests/live_config.rs`:
      `a_changed_session_backend_applies_to_new_requests`,
      `a_changed_queue_worker_count_starts_and_stops_workers_after_their_current_job`,
      `a_changed_control_socket_moves_the_control_endpoint` and
      `a_changed_server_timeout_applies_to_the_next_connection`. Each needs its subsystem to rebuild
      from the published snapshot first; the publish step is `Process::published` at
      `crates/nvs-cli/src/control.rs:195`.
- [ ] **`watching_the_configuration_costs_no_request_a_filesystem_call`** in
      `crates/nvs-cli/tests/live_config.rs`: the check runs on its own thread
      (`crates/nvs-cli/src/control.rs:393`), and the request path reads only `Current::load`.

## Backlog

- `queue.max_attempts` is proved by the value the push reads; the copy kept in the job row is not
  read back (`crates/nvs-cli/tests/live_config.rs`, the queue case).
- `[[app]]` `root` and `entry` are matched once per snapshot against the entry named on the
  command line (`crates/nvs-config/src/snapshot.rs:172`), so a mount-table server's rows never
  match an `[[app]]` block (not checked live).
- `[limits] wall_time`, `[limits] max_tasks` and `[http.errors] detail` have no reader in the
  server (`crates/nvs-server/src/schedule.rs:490`, `crates/nvs-config/src/mode.rs:91`).
- A featureless build (`--no-default-features`) ignores an exporter a reload adds, where the boot
  refuses one; nothing logs it (`crates/nvs-cli/src/serve.rs`, the exporter block in
  `serve_on_worker`).
- The `[[extension]]` pin is not checked against the file on disk anywhere in `nvs-config`, though
  `rule:config/the-config-is-an-immutable-snapshot` says a reload verifies it (not checked beyond a
  search for `sha256` in `crates/nvs-config/src`).
