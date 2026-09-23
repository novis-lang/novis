# Handoff

## State

**Side goal `restart-free` — a running server takes every code change without a restart, and every
config change it can.** ADR 0218 (source revalidation, Stages 2 to 4) is on disk with the five rules it
modifies; each fragment ends in a **What is on disk** paragraph the landing session shrinks.

The unit is keyed on the whole program. A compile, failed or not, records a `Trace`
(`crates/nvs-cli/src/script.rs`): every file the front end read, every path it missed, the `autoload`
probes and every directory a discovery scan listed. The table keeps, per path, the unit in force and
the one it replaced (`PathEntry::replaced`), plus at most one failure. `Compiler::traces` holds one
`Trace` per kept unit under its entry-file content, with the one that still describes the disk marked
current (`Traced`), so a reverted edit is a pointer swap, whether it was to the entry file or to a
`require`d one. The check still runs inside the resolve, on the request path.

`crates/nvs-cli/tests/live_edit.rs` has nine of the check's eleven tests, all passing. Still
unwritten: `a_queue_job_and_a_scheduled_fire_run_the_edited_code`,
`a_running_websocket_keeps_the_code_it_started_with`. The revert test counts a `W1011` warning on
standard error, because every compile writes it; nothing outside the process can read
`Compiler::compiles`.

One call in ADR 0218 is not the user's and is not confirmed: a mount re-expansion that meets a match
boot would refuse logs it and leaves it out (§ 9). Keeping the replaced unit (§ 7, § 8) is on disk
and was also my call. The startup table is `dispatch`, `static`, `settle`; `settle` does not exist
yet. `rule:packaging/autoload-probes-fold-into-the-cache-key` stays `designed` until the check leaves
the request path.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`), which
nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main checkout.

## Next group

**Stage 2: every file a program reached is watched** — one file set: `crates/nvs-cli/tests/live_edit.rs`,
`crates/nvs-cli/src/serve.rs`.

- [ ] **A queue job and a scheduled fire run the edited code** — both resolve their program through the
      same `Compiler` as a request (`rule:config/an-edit-reaches-the-next-request-without-a-restart`).
      Lands with `a_queue_job_and_a_scheduled_fire_run_the_edited_code` beside
      `crates/nvs-cli/tests/live_edit.rs:475`; the in-process schedule fixture is
      `crates/nvs-cli/src/serve.rs:2703`, and the queue worker is `crates/nvs-cli/src/worker.rs`.
- [ ] **A running WebSocket keeps the code it started with** — a connection isolate owns its unit
      through its own `Arc` (`rule:concurrency/connection-bounds-are-finite`, module doc of
      `crates/nvs-cli/src/script.rs:148`). Lands with `a_running_websocket_keeps_the_code_it_started_with`
      in `crates/nvs-cli/tests/live_edit.rs:475`; the upgrade is served from
      `crates/nvs-server/src/socket.rs`.

## Backlog
- Stage 2's other check: `validate = "never"` does not load and names `mtime` and `hash`, and `mtime` is
  the default in both modes — `crates/nvs-config/src/cache.rs`, `rule:config/opcache-revalidation-is-system-class`.
- The background check, `settle` and the link re-resolve (Stage 3) — `rule:config/an-edit-reaches-the-next-request-without-a-restart`.
