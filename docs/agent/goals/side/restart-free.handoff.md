# Handoff

## State

**Side goal `restart-free` — a running server takes every code change without a restart, and every
config change it can.** ADR 0218 (source revalidation, Stages 2 to 4) is on disk with the five rules it
modifies; each fragment ends in a **What is on disk** paragraph the landing session shrinks.

The unit is keyed on the whole program. A compile, failed or not, records a `Trace`
(`crates/nvs-cli/src/script.rs:231`): every file the front end read with its stamp and digest, every
path it missed (`SourceMap::missed`, fed by `SourceMap::load` and the `require` walk), and the
`autoload` probes. `crate::front_end_looking` and `Looked` (`crates/nvs-cli/src/main.rs:1747`) hand
that list back on both exits. The check still runs inside the resolve, on the request path.

`crates/nvs-cli/tests/live_edit.rs` has seven of the check's eleven tests, all passing: the entry-file
edit, the broken edit and its fix, a required-file edit, an autoloaded-class edit, a deleted required
file (and its restore), a new class file under a root, and a shadowing file. Still unwritten:
`a_new_module_under_a_discovery_directory_joins_implementing_without_a_restart`,
`a_reverted_edit_is_answered_from_the_unit_already_compiled`,
`a_queue_job_and_a_scheduled_fire_run_the_edited_code`, `a_running_websocket_keeps_the_code_it_started_with`.

Two calls in ADR 0218 are not the user's and are not confirmed: the table keeps, per path, the unit in
force and the one it replaced (§ 7, § 8); a mount re-expansion that meets a match boot would refuse
logs it and leaves it out (§ 9). The startup table is `dispatch`, `static`, `settle`; `settle` does not
exist yet.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`), which
nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main checkout.

## Next group

**Stage 2: every file a program reached is watched** — one file set: `crates/nvs-cli/src/script.rs`,
`crates/nvs-cli/tests/live_edit.rs`, `crates/nvs-hir/src/autoload.rs`.

- [ ] **A discovery query's directories join the check** — every directory the `discover` scan
      listed (`crates/nvs-hir/src/autoload.rs:567`, `listing` at `crates/nvs-hir/src/autoload.rs:621`)
      goes into the trace beside the probes, with its stamp, and the sorted discovered names hash into
      the key (`rule:packaging/autoload-probes-fold-into-the-cache-key`). Carry the list out through
      `Looked` (`crates/nvs-cli/src/main.rs:1747`) into `Trace` (`crates/nvs-cli/src/script.rs:231`)
      and check it in `revalidate_trace` (`crates/nvs-cli/src/script.rs:724`). Lands with
      `a_new_module_under_a_discovery_directory_joins_implementing_without_a_restart` in
      `crates/nvs-cli/tests/live_edit.rs`.
- [ ] **A reverted edit is a pointer swap** — the table keeps, per path, the unit in force and the one
      it replaced (ADR 0218 § 7, § 8); `record` at `crates/nvs-cli/src/script.rs:852` sweeps every
      other generation today, and `traces` holds one trace per entry content. Lands with
      `a_reverted_edit_is_answered_from_the_unit_already_compiled`.

## Backlog

- `a_queue_job_and_a_scheduled_fire_run_the_edited_code` and `a_running_websocket_keeps_the_code_it_started_with` — stage 2, `crates/nvs-cli/src/worker.rs`, goal prose stage 2.
- The background check, `settle` and the link re-resolve — stages 3 and 4, `rule:config/an-edit-reaches-the-next-request-without-a-restart`.
