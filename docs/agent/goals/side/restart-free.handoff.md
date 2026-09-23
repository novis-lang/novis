# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4). It is on disk with the
five rules it modifies. Each fragment ends in a **What is on disk** paragraph, and the session that
lands the rest shrinks it.

**Stages 2, 3 and 4 are complete. Stage 5 has begun.** `serve::mounts::Rescan` now runs for every
`nvs serve`, a named file over no mount table included. Each pass reads `[[app]] origin` from the
published tree (`nvs_config::Current`) and, where it moved, folds it into the rows the last expansion
gave and asks the origin check of every changed row. `[[server.mount]]` and `[server] root` are still
read from the boot tree (`[server]` is `Boot`-class). `crates/nvs-cli/tests/live_config.rs` holds the
Stage 5 harness: `Server::start(case, config, files)` writes `[control] socket` itself, and
`Server::reload(config)` rewrites `nvs.toml` and runs `nvs ctl reload`. Its first case,
`a_changed_app_origin_reaches_the_mount_rows`, passes, and fails with the fold disabled.

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

**Stage 5: every reloadable key really reloads** — one file set: `crates/nvs-cli/src/serve.rs`,
`crates/nvs-cli/tests/live_config.rs`, and `nvs_server`'s `Secure` and `Cors`.

- [ ] **`Secure` and `Cors` are derived per published snapshot** (goal § Stage 5,
      `rule:http-server/cors-is-closed-until-origins-are-named`). The process builds them once at
      `crates/nvs-cli/src/serve.rs:290` (`Serving::live`). Tests
      `a_changed_http_headers_block_reaches_the_next_response` and
      `a_changed_cors_block_reaches_the_next_response` in the same file; `Answer`
      (`crates/nvs-cli/tests/live_config.rs:41`) needs the response head for them.
- [ ] **A reload that moves `[[app]] origin` names it as applied**
      (`rule:config/a-reload-names-what-it-could-not-apply`). The live case's server logged
      `"applied":[]` for that reload. The report is built by `nvs_config::control::reload`, called
      at `crates/nvs-cli/src/control.rs:151`.

## Backlog

- Stage 5's other keys: `[opcache]`, the admission ceiling, `[[schedule]]`, queue `visibility`, metrics and trace exporters, and the census — `docs/agent/goals/side/restart-free.md` § Stage 5.
- Stage 6, the configuration applies itself, and `[[server.mount]]` / `[server] root` following a reload — same file, § Stage 6, and a second decision record.
- A module removed from the mount table keeps its path entry in the compiler, checked on every pass until the process ends — `crates/nvs-cli/src/script.rs` (`Compiler::revalidate`).
- The ten-thousand-edit test costs one debug compile per edit; a cheaper compile path would shorten it — `crates/nvs-cli/src/script.rs`.
