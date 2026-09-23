# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4). It is on disk with the
five rules it modifies. Each fragment ends in a **What is on disk** paragraph, and the session that
lands the rest shrinks it.

**Stages 2, 3 and 4 are complete. Stage 5 is under way.** `[[app]] origin`, `[http.headers]` and
`[http.cors]` now reload. The request door in `nvs_server::serve` takes the snapshot once at the
request's start, and `Serving::policy` derives `Secure` and `Cors` from it the first time a request
runs under a new snapshot. `crates/nvs-cli/tests/live_config.rs` holds the Stage 5 harness:
`Server::start(case, config, files)`, `Server::reload(config)`, and `get_with`/`awaits_with`, which
send extra request headers. `Answer::header` reads a response header. Each of its three cases fails
with its fix disabled.

Calls in ADR 0218 that are mine and not confirmed with the user: a mount re-expansion that meets a
match boot would refuse logs it and leaves it out (§ 9), keeping the replaced unit (§ 7, § 8), and
the `FLOOR` of 10ms between two watcher passes. Also mine: a row whose entry does not compile is
kept in the table, a `[server] root` that vanishes keeps the table as it stands, and a reload that
removes `[[app]] origin` from a row whose unit calls `urlAbsolute` leaves that row out (404) rather
than refusing the reload.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 5: every reloadable key really reloads** — one file set: `crates/nvs-cli/src/control.rs`,
`crates/nvs-cli/src/script.rs`, and `crates/nvs-cli/tests/live_config.rs`.

- [ ] **A reload that moves `[[app]] origin` names it as applied**
      (`rule:config/a-reload-names-what-it-could-not-apply`). The live case's server logged
      `"applied":[]` for that reload. The report is built by `nvs_config::control::reload`, called
      from `crates/nvs-cli/src/control.rs:191`. Assert the applied list in
      `a_changed_app_origin_reaches_the_mount_rows` (`Server::reload` returns what `nvs ctl
      reload` printed).
- [ ] **`[opcache]` reloads into the unit cache** (`rule:config/opcache-revalidation-is-system-class`,
      `rule:config/reloadability-is-its-own-field`). The compiler reads `validate`,
      `revalidate_freq` and `settle` once, at `crates/nvs-cli/src/script.rs:496`; the watcher reads
      them at `crates/nvs-cli/src/script.rs:665` and `crates/nvs-cli/src/script.rs:575`. Test
      `a_changed_opcache_block_reaches_the_unit_cache` in `crates/nvs-cli/tests/live_config.rs`.

## Backlog

- Stage 5's other keys: the admission ceiling, `[[schedule]]`, queue `visibility`, metrics and trace exporters, and the census — `docs/agent/goals/side/restart-free.md` § Stage 5.
- `crate::metrics::meter_this_core` reads the snapshot once per core at start (`crates/nvs-server/src/serve.rs`), which the metrics/trace case will meet — same goal file, § Stage 5.
- Stage 6, the configuration applies itself, and `[[server.mount]]` / `[server] root` following a reload — same file, § Stage 6, and a second decision record.
- A module removed from the mount table keeps its path entry in the compiler, checked on every pass until the process ends — `crates/nvs-cli/src/script.rs`.
- The ten-thousand-edit test costs one debug compile per edit; a cheaper compile path would shorten it — `crates/nvs-cli/src/script.rs`.
