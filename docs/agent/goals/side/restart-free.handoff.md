# Handoff

## State

**Side goal `restart-free` — a running server takes every code change without a restart, and every
config change it can.** ADR 0218 (source revalidation, Stages 2 to 4) is on disk with the five rules it
modifies; each fragment ends in a **What is on disk** paragraph the landing session shrinks.

**Stage 2 is complete.** All eleven `crates/nvs-cli/tests/live_edit.rs` tests pass, including a queue
job and a scheduled fire running the edited script (one file, SQLite queue migrated by
`Server::start_after`, a `* * * * *` entry, so that case waits up to a minute) and a WebSocket keeping
its code while a new connection gets the edit (`Server::websocket`, short text frames only). The
`nvs-config` `validate` tests of stage 2 pass too.

The unit is keyed on the whole program, and a compile records a `Trace` (`crates/nvs-cli/src/script.rs`).
The table keeps, per path, the unit in force and the one it replaced, so a reverted edit is a pointer
swap. The check still runs inside the resolve, on the request path, gated by `revalidate_freq`.

One call in ADR 0218 is not the user's and is not confirmed: a mount re-expansion that meets a match
boot would refuse logs it and leaves it out (§ 9). Keeping the replaced unit (§ 7, § 8) was also my
call. `rule:packaging/autoload-probes-fold-into-the-cache-key` stays `designed` until the check leaves
the request path.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`), which
nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main checkout.

## Next group

**Stage 3: the check leaves the request path** — one file set: `crates/nvs-config/src/directive.rs`,
`crates/nvs-config/src/cache.rs`, `crates/nvs-config/src/tree.rs`, `crates/nvs-config/tests/snapshot.rs`.

- [ ] **`[opcache] settle` exists, `System` and reloadable, `"1s"` in production and `"100ms"` in
      development** (`rule:config/opcache-revalidation-is-system-class`,
      `rule:config/a-startup-default-is-never-flipped`). The `opcache` block row is
      `crates/nvs-config/src/directive.rs:230`, the written key sits beside `revalidate_freq` at
      `crates/nvs-config/src/tree.rs:1357`, its reader beside the interval at
      `crates/nvs-config/src/cache.rs:462`, and the startup rows are named at
      `crates/nvs-config/src/mode.rs:8`. Tests `settle_defaults_to_one_second_in_production_and_100ms_in_development`
      and `settle_is_system_class_and_reloadable` go beside `crates/nvs-config/tests/snapshot.rs:685`;
      the template line goes in `crates/nvs-config/src/default.toml:859`.
- [ ] **A background check on the compile pool replaces the resolve-time one**
      (`rule:config/an-edit-reaches-the-next-request-without-a-restart`): the check today is
      `crates/nvs-cli/src/script.rs:807`, and the stage's seven `live_edit` tests are listed in
      `docs/agent/goals/side/restart-free.toml:156`.

## Backlog

- Stage 4, mounts follow the disk — `docs/agent/goals/side/restart-free.md` § Stage 4.
- Stages 5 and 6, configuration apply, need their own decision record — the goal's standing decisions.
