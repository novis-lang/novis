# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4). It is on disk with the
five rules it modifies. Each fragment ends in a **What is on disk** paragraph, and the session that
lands the rest shrinks it.

**Stages 2, 3 and 4 are complete. Stage 5 has one check left, the census.** `[[app]] origin`,
`[http.headers]`, `[http.cors]`, `[opcache]`, `limits.memory`, the queue worker's snapshot, the
`[[schedule]]` roster and both observability blocks now reload. The exporters are owned by
`crates/nvs-cli/src/serve/exporters.rs` (module doc says how): a watcher task on the ticking core
re-resolves `[metrics]` and `[trace]` every second and replaces an exporter under a drain of its
own, binding a new scrape socket before the old one closes. Each core re-meters its registry when
an accepted connection sees a new snapshot (`nvs_server::serve_on_this_core`). All eight cases in
`crates/nvs-cli/tests/live_config.rs` fail with their fix disabled.

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
exporter with no endpoint) is logged and keeps the running exporter, and a reload that removes
`[metrics]` leaves each core's registry counting with nothing shipping it.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 5: every reloadable key really reloads** — one file set: `crates/nvs-config/src/directive.rs`,
`crates/nvs-config/tests/directives.rs`, `docs/decisions/`, `docs/rules/config/`.

- [ ] **The census** `every_directive_has_a_live_apply_proof_or_a_restart_proof` in `nvs-config`
      (`rule:config/reloadability-is-its-own-field`). Every row of the registry at
      `crates/nvs-config/src/directive.rs:90` names a `live_config.rs` case that proves it applies
      live, or is `Apply::Boot` and has a proof that the reload report names it
      (`rule:config/a-reload-names-what-it-could-not-apply`). A row added without either fails.
      Test file `crates/nvs-config/tests/directives.rs`.
- [ ] **The configuration-apply decision record** — next free number on `main` at the moment it
      is written (`git -C D:/mwl ls-tree main docs/decisions/`). `changes.modifies` names
      `config/reloadability-is-its-own-field` and
      `http-server/admission-is-arithmetic-not-a-number`; each rule's `because` gains the number
      in the same commit, then `python tools/rules.py --render`. The tradeoffs to state are the
      goal's § *Standing decisions* list, anchored at `docs/agent/goals/side/restart-free.md:192`.

## Backlog

- Stage 6: the server checks its own configuration files, and `Boot` shrinks to three keys
  (`docs/agent/goals/side/restart-free.md` § Stage 6).
- A featureless build (`--no-default-features`) ignores an exporter a reload adds, where the boot
  refuses one; nothing logs it (`crates/nvs-cli/src/serve.rs`, the exporter block in
  `serve_on_worker`).
