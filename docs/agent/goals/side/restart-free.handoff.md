# Handoff

## State

**Side goal `restart-free` — a running server takes every code change without a restart, and every
config change it can.** ADR 0218 (source revalidation, Stages 2 to 4) is on disk with the five rules it
modifies; each fragment ends in a **What is on disk** paragraph the landing session shrinks.

**Stage 2 is complete**, and stage 3 has started. `[opcache] settle` exists: `System` and `Reload`
through the `opcache` block row, read into `nvs_config::cache::Revalidation::settle`, and when unwritten
it is the startup row of the mode `[mode] default` names (`SETTLE_PRODUCTION` 1s, `SETTLE_DEVELOPMENT`
100ms, beside the reader in `crates/nvs-config/src/cache.rs`). The stage-3 `nvs-config` check passes.
Nothing waits on `settle` yet.

The unit is keyed on the whole program, and a compile records a `Trace` (`crates/nvs-cli/src/script.rs`).
The table keeps, per path, the unit in force and the one it replaced, so a reverted edit is a pointer
swap. The check still runs inside the resolve, on the request path, gated by `revalidate_freq`.

One call in ADR 0218 is not the user's and is not confirmed: a mount re-expansion that meets a match
boot would refuse logs it and leaves it out (§ 9). Keeping the replaced unit (§ 7, § 8) was also my
call. `rule:packaging/autoload-probes-fold-into-the-cache-key` stays `designed` until the check leaves
the request path. `rule:config/a-startup-default-is-never-flipped`'s `dispatch` and `static` rows are
still not derived from the mode (an unwritten switch is closed in both, `crates/nvs-server/src/mount.rs:35`);
that is outside this goal.

`verify.py`'s `extension` leg fails on `tsc` not found (missing `editors/vscode/node_modules`), which
nothing here touches. `tests/db/ca.crt` is a git-ignored fixture copied in from the main checkout.

## Next group

**Stage 3: the check leaves the request path** — one file set: `crates/nvs-cli/src/script.rs`,
`crates/nvs-cli/tests/live_edit.rs`.

- [ ] **A background check on the compile pool replaces the resolve-time one**
      (`rule:config/an-edit-reaches-the-next-request-without-a-restart`, ADR 0218). The gate a request
      passes today is `crates/nvs-cli/src/script.rs:618`, the trace check it calls is
      `crates/nvs-cli/src/script.rs:807`, and the policy it holds is `crates/nvs-cli/src/script.rs:480`
      (now carrying `settle`). A request becomes a map lookup; a task checks every loaded program once
      per `revalidate_freq`. Where that task is spawned is not checked. Tests
      `a_request_makes_no_filesystem_call_to_revalidate`, `a_change_is_compiled_before_it_is_swapped_in`
      and `an_idle_server_takes_an_edit_within_revalidate_freq_and_settle` go beside
      `crates/nvs-cli/tests/live_edit.rs:721`.
- [ ] **A change waits for `settle`, and a file that moves during the compile discards it** — tests
      `a_copy_that_pauses_less_than_settle_is_compiled_once_from_the_finished_tree` and
      `a_file_that_changes_during_a_compile_discards_that_compile`, in
      `crates/nvs-cli/tests/live_edit.rs:721`.
- [ ] **The entry path is resolved through every link before and after a compile** — test
      `a_link_switch_during_a_compile_never_builds_a_program_from_both_releases`
      (`crates/nvs-cli/tests/live_edit.rs:721`).
- [ ] **Units stay bounded** — test `units_held_stay_bounded_after_ten_thousand_edits`
      (`crates/nvs-cli/src/script.rs:237`).

## Backlog

- When the check leaves the request path, shrink the **What is on disk** paragraphs of
  `rule:config/an-edit-reaches-the-next-request-without-a-restart`,
  `rule:config/opcache-revalidation-is-system-class` and
  `rule:packaging/autoload-probes-fold-into-the-cache-key` (and ship the last).
- Stages 4 to 6 of `docs/agent/goals/side/restart-free.md` follow stage 3.
