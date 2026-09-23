# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4). It is on disk with the
five rules it modifies. Each fragment ends in a **What is on disk** paragraph, and the session that
lands the rest shrinks it.

**Stages 2, 3 and 4 are complete. Stage 5 has one check left, the census, and its scaffold is on
disk.** `every_row_names_its_apply_proof_or_is_listed_as_owing_one` in
`crates/nvs-config/tests/directives.rs` maps every registry row to its proof: a `Reload` row names a
`live_config.rs` case, a `Boot` row names `every_boot_row_a_reload_changes_is_named_and_keeps_its_running_value`
in `crates/nvs-config/tests/snapshot.rs`. Twelve `Reload` rows are in `OWED_LIVE_PROOF` and still need
a live case. `every_request_read_directive_takes_the_reloaded_value_in_the_next_request` in
`crates/nvs-cli/tests/live_config.rs` proves 17 rows through `Core\Config::get` in a real request. It
fails when the reload writes the boot's values. An audit found every one of those rows read out of the
request's own snapshot per call, so reading it back is the value its reader sees.

**The census proper is not written yet.** When `OWED_LIVE_PROOF` is empty, rename the scaffold test to
`every_directive_has_a_live_apply_proof_or_a_restart_proof` and delete the list.

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
1s `SCHEDULE_POLL`, a `fleet` entry a reload adds to a boot that opened no lease is noted and not
armed, an exporter change that cannot be followed (a scrape port that will not bind, an `otlp`
exporter with no endpoint) is logged and keeps the running exporter, a reload that removes
`[metrics]` leaves each core's registry counting with nothing shipping it, and proving a
request-read row by reading it back through `Core\Config::get`.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 5: every reloadable key really reloads** — one file set: `crates/nvs-cli/tests/live_config.rs`,
`crates/nvs-config/tests/directives.rs`, `crates/nvs-cli/src/script.rs`.

- [ ] **Live cases for the owed rows** (`rule:config/reloadability-is-its-own-field`). Each row in
      `OWED_LIVE_PROOF` at `crates/nvs-config/tests/directives.rs:335` gets a case in
      `crates/nvs-cli/tests/live_config.rs:860` and moves into `APPLY_PROOFS`. `mode.default` and
      `mode.ceiling` need the harness's `PRODUCTION` prefix made optional (`controlled`, line 332).
      `capabilities`: a grant a reload removes makes a call fail. `http.csrf_key(_file)`: a token
      signed with the old key is refused at the door. `http.client.proxy`, `log.handler`,
      `log.target`, `debug.inline` and `queue.max_attempts` (read per push, then kept in the job
      row) each need an observable. `include`: an included file's key changes after a reload.
- [ ] **`[[extension]]` on-disk artifact cache keeps the boot's `env_hash`**, so a reload that
      changes the set rekeys the unit table but not the disk cache. The anchor is
      `crates/nvs-cli/src/cache.rs:1640` (`Cache::new` with `env_hash` of the boot), beside the
      unit table's rekeyed digest at `crates/nvs-cli/src/script.rs:568`. Fix it, then
      prove `extension` live. `rule:config/the-extension-set-is-in-every-unit-key`.
- [ ] **The census** `every_directive_has_a_live_apply_proof_or_a_restart_proof`: rename the
      scaffold at `crates/nvs-config/tests/directives.rs:366` once the owed list is empty.
- [ ] **The configuration-apply decision record** — next free number on `main` at the moment it
      is written (`git -C D:/mwl ls-tree main docs/decisions/`). `changes.modifies` names
      `config/reloadability-is-its-own-field` and
      `http-server/admission-is-arithmetic-not-a-number`; each rule's `because` gains the number
      in the same commit, then `python tools/rules.py --render`. The tradeoffs to state are the
      goal's § *Standing decisions* list, anchored at `docs/agent/goals/side/restart-free.md:192`.

## Backlog

- Stage 6: the server checks its own configuration files, and `Boot` shrinks to three keys
  (`docs/agent/goals/side/restart-free.md` § Stage 6). Each row that leaves `Boot` then moves
  from `BOOT_CHANGES` in `crates/nvs-config/tests/snapshot.rs` to a live case.
- `[[app]]` `root` and `entry` are matched once per snapshot against the entry named on the
  command line (`crates/nvs-config/src/snapshot.rs:172`), so a mount-table server's rows never
  match an `[[app]]` block (not checked live).
- `[limits] wall_time`, `[limits] max_tasks` and `[http.errors] detail` have no reader in the
  server (`crates/nvs-server/src/schedule.rs:490`, `crates/nvs-config/src/mode.rs:91`).
- A featureless build (`--no-default-features`) ignores an exporter a reload adds, where the boot
  refuses one; nothing logs it (`crates/nvs-cli/src/serve.rs`, the exporter block in
  `serve_on_worker`).
