# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4). It is on disk with the
five rules it modifies. Each fragment ends in a **What is on disk** paragraph, and the session that
lands the rest shrinks it.

**Stages 2, 3 and 4 are complete.** A request for a path with a pointer does one map lookup and
makes no file-system call (`Compiler::compiled`). `Compiler::revalidate` runs on one
`nvs-revalidate` thread (`script::watch`), from `nvs serve` and `nvs run`. `nvs serve` over a
`[[server.mount]]` table also runs `serve::mounts::Rescan::pass` on that thread: it stamps every
directory the expansion looked in, waits `settle` after one moves, and expands again with
`nvs_config::mount::expand_again`, which leaves a refused match out. Each core rebuilds its
`Table` when the shared `serve::mounts::Mounts` generation moves. The rescan reads the **boot**
snapshot, so a reload that changes `[[server.mount]]`, `[server] root` or `[[app]] origin` does not
reach the rows (the rule fragment's **What is on disk** says so).

Calls in ADR 0218 that are mine and not confirmed with the user: a mount re-expansion that meets a
match boot would refuse logs it and leaves it out (§ 9), keeping the replaced unit (§ 7, § 8), and
the `FLOOR` of 10ms between two watcher passes. Also mine: a row whose entry does not compile is
kept in the table, and a `[server] root` that vanishes keeps the table as it stands.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`),
which nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main
checkout.

## Next group

**Stage 5: every reloadable key really reloads** — one file set: `crates/nvs-cli/src/serve.rs`,
`crates/nvs-cli/src/serve/mounts.rs`, `crates/nvs-cli/src/control.rs`, and a new
`crates/nvs-cli/tests/live_config.rs` whose `Server` harness is a copy of `live_edit.rs`'s.

- [ ] **`[[app]] origin` is folded into the mount rows again on publish** (goal § Stage 5,
      `rule:routing/an-origin-is-per-mount-and-checked-at-boot`). `Rescan` holds the boot snapshot
      at `crates/nvs-cli/src/serve/mounts.rs:227`; give it `nvs_config::Current` and fold
      `serve::fall_back_to` from the published tree, publishing the rows when the origin moved.
      The reload is `crates/nvs-cli/src/control.rs:191`. Test
      `a_changed_app_origin_reaches_the_mount_rows` in the new `live_config.rs`.
- [ ] **`Secure` and `Cors` are derived per published snapshot** (goal § Stage 5). The process
      builds them once at `crates/nvs-cli/src/serve.rs:290` (`Serving::live`). Tests
      `a_changed_http_headers_block_reaches_the_next_response` and
      `a_changed_cors_block_reaches_the_next_response`, same file.

## Backlog

- Stage 5's other keys: `[opcache]`, the admission ceiling, `[[schedule]]`, queue `visibility`, metrics and trace exporters, and the census — `docs/agent/goals/side/restart-free.md` § Stage 5.
- Stage 6, the configuration applies itself — same file, § Stage 6, and a second decision record.
- A module removed from the mount table keeps its path entry in the compiler, checked on every pass until the process ends — `crates/nvs-cli/src/script.rs` (`Compiler::revalidate`).
- The ten-thousand-edit test costs one debug compile per edit; a cheaper compile path would shorten it — `crates/nvs-cli/src/script.rs`.
