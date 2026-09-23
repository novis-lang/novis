# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4). It is on disk with the
five rules it modifies. Each fragment ends in a **What is on disk** paragraph, and the session that
lands the rest shrinks it.

**Stages 2 to 5 are complete.** The census
`every_directive_has_a_live_apply_proof_or_a_restart_proof` in
`crates/nvs-config/tests/directives.rs` maps every registry row to exactly one proof. A `Reload` row
names a case in `crates/nvs-cli/tests/live_config.rs`, and a `Boot` row names
`every_boot_row_a_reload_changes_is_named_and_keeps_its_running_value` in
`crates/nvs-config/tests/snapshot.rs`. No row is owed.

**ADR 0219 (configuration apply, Stages 5 and 6) is on disk**, and every rule its
`changes.modifies` names carries `0219` in its `because`. The fragments whose text Stage 6 makes
wrong still describe the code as it is (the wide `Boot` set, a reload pushed by the operator). Item 3
below lists them, and each is rewritten by the slice that lands its behaviour. It is the last record
number this goal takes.

Calls that are mine and not confirmed with the user. In ADR 0219: the config check is a fixed two
seconds and not a directive (§ 4), a moved stamp must hold for one more check before it is applied
(§ 4), a resource that cannot be built keeps its running value while the rest of the snapshot
publishes (§ 7), and a changed session backend does not carry sessions over (§ 7). In ADR 0218: a
mount re-expansion that meets a match boot would refuse logs it and leaves it out (§ 9), keeping
the replaced unit (§ 7, § 8), and the `FLOOR` of 10ms between two watcher passes. From Stage 5: a
row whose entry does not compile is kept in the table, a `[server] root` that vanishes keeps the
table as it stands, a reload that removes `[[app]] origin` from a row whose unit calls
`urlAbsolute` leaves that row out (404), a roster change is reported as the one key `app`, a
lowered admission ceiling keeps every admitted request counted, a reload that removes `[queue]`
leaves a running worker on the boot's `visibility`, the 1s `SCHEDULE_POLL`, a `fleet` entry a
reload adds to a boot that opened no lease is noted and not armed, an exporter change that cannot
be followed is logged and keeps the running exporter, a reload that removes `[metrics]` leaves each
core's registry counting with nothing shipping it, and proving a request-read row by reading it
back through `Core\Config::get`.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 6: the configuration applies itself** — one file set: `crates/nvs-cli/src/control.rs`,
`crates/nvs-config/src/directive.rs`, `crates/nvs-config/src/snapshot.rs`,
`crates/nvs-config/tests/snapshot.rs`.

- [ ] **The server checks its configuration files** (`rule:config/the-config-is-an-immutable-snapshot`,
      ADR 0219 § 4 and § 5). A timer off the request path checks, every two seconds, the stamp of
      every path the last published resolution read, absent optional includes too. A moved stamp
      that holds for one more check is published through the function `nvs ctl reload` uses, at
      `crates/nvs-cli/src/control.rs:164`, under the same lock. A file that does not validate is
      logged once per content with its line. A changed restart key is logged with both values and
      listed by `nvs ctl status` (`crates/nvs-cli/src/ctl.rs:75`). Rewrite the fragments of
      `rule:config/the-config-is-an-immutable-snapshot` ("a reload is pushed by the operator") and
      `rule:config/a-reload-names-what-it-could-not-apply` in the same slice.
- [ ] **`Boot` shrinks to three keys** (`rule:config/reloadability-is-its-own-field`, ADR 0219 § 6
      and § 7). Split the `server` row in `crates/nvs-config/src/directive.rs` into one row per key;
      each row that leaves `Boot` moves out of `BOOT_CHANGES` at
      `crates/nvs-config/tests/snapshot.rs:457` into a live case. The fragments that still say
      `Boot` and are rewritten as their key lands: `reloadability-is-its-own-field`,
      `http-server/the-server-block-is-boot-class` (title and body; the id stays),
      `config/a-startup-default-is-never-flipped` (the `dispatch` and `static` rows),
      `http-server/a-request-resolves-in-five-steps`, `http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot`,
      `concurrency/connection-bounds-are-finite`, `routing/an-origin-is-per-mount-and-checked-at-boot`
      and `config/one-local-control-socket` (the endpoint moves on a reload).

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
