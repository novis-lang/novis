# Handoff

## State

**Side goal `restart-free` — a running server takes every code change without a restart, and every
config change it can.** ADR 0218 (source revalidation, Stages 2 to 4) is on disk with the five rules it
modifies; each fragment ends in a **What is on disk** paragraph the landing session shrinks.

The unit is keyed on the whole program. A compile, failed or not, records a `Trace`
(`crates/nvs-cli/src/script.rs:233`): every file the front end read with its stamp and digest, every
path it missed, the `autoload` probes, and every directory a discovery scan listed with its stamp and
the names it held (`nvs_hir::autoload::Listing`; a `discover` glob's base and every directory
`implementing` walked). `discovery_hash` in `nvs-config`'s `cache.rs` folds the listings into the key's
probe field. The check still runs inside the resolve, on the request path.

`crates/nvs-cli/tests/live_edit.rs` has eight of the check's eleven tests, all passing. Still
unwritten: `a_reverted_edit_is_answered_from_the_unit_already_compiled`,
`a_queue_job_and_a_scheduled_fire_run_the_edited_code`, `a_running_websocket_keeps_the_code_it_started_with`.

Two calls in ADR 0218 are not the user's and are not confirmed: the table keeps, per path, the unit in
force and the one it replaced (§ 7, § 8); a mount re-expansion that meets a match boot would refuse
logs it and leaves it out (§ 9). The startup table is `dispatch`, `static`, `settle`; `settle` does not
exist yet. `rule:packaging/autoload-probes-fold-into-the-cache-key` stays `designed` until the check
leaves the request path.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`), which
nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main checkout.

## Next group

**Stage 2: every file a program reached is watched** — one file set: `crates/nvs-cli/src/script.rs`,
`crates/nvs-cli/tests/live_edit.rs`.

- [ ] **A reverted edit is a pointer swap** — the table keeps, per path, the unit in force and the one
      it replaced (ADR 0218 § 7, § 8, `rule:config/an-edit-reaches-the-next-request-without-a-restart`);
      `record` at `crates/nvs-cli/src/script.rs:884` sweeps every other generation today, and `traces`
      holds one trace per entry content. Lands with
      `a_reverted_edit_is_answered_from_the_unit_already_compiled` in `crates/nvs-cli/tests/live_edit.rs`,
      beside `crates/nvs-cli/tests/live_edit.rs:415`; `Compiler::compiles` counts front-end runs, which is
      what the test can read to prove no compile happened.

## Backlog

- `a_queue_job_and_a_scheduled_fire_run_the_edited_code` and `a_running_websocket_keeps_the_code_it_started_with` — stage 2, `crates/nvs-cli/src/worker.rs`, goal prose stage 2.
- The background check, `settle` and the link re-resolve — stages 3 and 4, `rule:config/an-edit-reaches-the-next-request-without-a-restart`.
