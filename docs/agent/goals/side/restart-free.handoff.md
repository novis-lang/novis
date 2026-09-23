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
`crates/nvs-config/tests/snapshot.rs`. No row is owed. The disk cache of compiled programs now moves
to the new `env_hash` on a reload that changes `[[extension]]` (`Compiler::rekey` in
`crates/nvs-cli/src/script.rs`), and `a_changed_extension_set_compiles_every_program_again` fails
without that.

**The configuration-apply decision record (Stages 5 and 6) is not written yet.** Its
`changes.modifies` must name `http-server/admission-is-arithmetic-not-a-number`, whose fragment
already says a reload recomputes the ceiling, and that rule's `because` must gain the record's
number in the same commit.

Calls in ADR 0218 that are mine and not confirmed with the user: a mount re-expansion that meets a
match boot would refuse logs it and leaves it out (§ 9), keeping the replaced unit (§ 7, § 8), and
the `FLOOR` of 10ms between two watcher passes. Also mine: a row whose entry does not compile is
kept in the table, a `[server] root` that vanishes keeps the table as it stands, a reload that
removes `[[app]] origin` from a row whose unit calls `urlAbsolute` leaves that row out (404) rather
than refusing the reload, a roster change is reported as the one key `app` rather than per block,
a lowered admission ceiling keeps every admitted request counted rather than cancelling any, a
reload that removes the `[queue]` block leaves a running worker on the boot's `visibility`, the
1s `SCHEDULE_POLL`, a `fleet` entry a reload adds to a boot that opened no lease is noted and not
armed, an exporter change that cannot be followed (a scrape port that will not bind, an `otlp`
exporter with no endpoint) is logged and keeps the running exporter, a reload that removes
`[metrics]` leaves each core's registry counting with nothing shipping it, and proving a
request-read row by reading it back through `Core\Config::get`. That read-back now also proves
`log.handler`, `log.target`, `debug.inline`, `http.client.proxy`, `mode.default`, `mode.ceiling`,
`include` and `queue.max_attempts` (the value the push read, not the job row's copy).

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 6: the configuration applies itself** — one file set: `crates/nvs-cli/src/control.rs`,
`crates/nvs-config/src/directive.rs`, `crates/nvs-config/src/snapshot.rs`,
`crates/nvs-config/tests/snapshot.rs`.

- [ ] **The configuration-apply decision record** — next free number on `main` at the moment it
      is written (`git -C D:/mwl ls-tree main docs/decisions/`; `0219` when this was written).
      `changes.modifies` names `config/reloadability-is-its-own-field` and
      `http-server/admission-is-arithmetic-not-a-number`; each rule's `because` gains the number
      in the same commit, then `python tools/rules.py --render`. The tradeoffs to state are the
      goal's § *Standing decisions* list, anchored at `docs/agent/goals/side/restart-free.md:192`.
- [ ] **The server checks its configuration files** (`rule:config/the-config-is-an-immutable-snapshot`,
      goal § Stage 6). A timer off the request path checks the stamp of every file the boot
      resolved and publishes through the function `nvs ctl reload` uses, at
      `crates/nvs-cli/src/control.rs:164`. A file that does not validate is logged with its line.
- [ ] **`Boot` shrinks to three keys** (`rule:config/reloadability-is-its-own-field`). Split the
      `server` row in `crates/nvs-config/src/directive.rs` into one row per key; each row that
      leaves `Boot` moves out of `BOOT_CHANGES` at `crates/nvs-config/tests/snapshot.rs:457` into
      a live case.

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
