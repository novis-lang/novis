# Handoff

## State

**Goal 7 — ADR 0131's temporary-directory sweep — has §§ 2-5 on disk, and stage 3 is green on both
sides.** The request-path pair lives beside the queue it is ordered against, in
`crates/nvs-runtime/src/deferred.rs`: the drain finds the directory the request was handed, and the
teardown behind it is what takes it away. `crates/nvs-runtime/src/ctx/hooks.rs:660` is the same claim
against § 3's CLI ending, so both of § 3's orderings are now asserted from the side that can observe
them.

`crate::sweep::at_script_end` still has exactly one caller — `crates/nvs-runtime/src/ctx/mod.rs:1172`,
inside `Ctx::drop` — which is why a cancelled request is swept at all: ADR 0106's worker survives the
request and drops the context it is left holding. The aborted case pins that the abort runs none of
§ 6's registrations while still owing the sweep.

**One test the goal's acceptance list names is not in the tree yet**:
`a_finished_scripts_temporary_dir_is_gone_from_the_owned_root`, stage 5, `-p nvs-cli`. Everything else
stages 2, 3, 4 and 5 name is on disk and green.

`[context] modules` now names this goal's own modules — `sweep.rs`, `deferred.rs`, `ctx/hooks.rs` — in
both `docs/agent/loop-goal.toml` and `docs/agent/goals/7-temp-sweep.toml`, closing the missing-map-line
gap the previous handoff reported.

## Next group

**Stage 5's last test, end to end through the CLI.** One file set: `crates/nvs-cli/src/script.rs`,
`crates/nvs-cli/src/tmp.rs`.

- [ ] **`a_finished_scripts_temporary_dir_is_gone_from_the_owned_root`** (0131 §§ 2-3, stage 5) — a
      whole script run under a configured `[io] temp_root`, and the root holding nothing afterwards:
      the end-to-end reading of what `-p nvs-runtime` already pins per context.
      `crates/nvs-cli/src/script.rs:602` is the run shape (`run_serving`, with the capability grant at
      `crates/nvs-cli/src/script.rs:542` — the playbook's bullet on a `spawn script` fixture owns the
      scheduler-and-reactor order it needs), and `crates/nvs-cli/src/tmp.rs:133` is the test module
      already building a root of its own to drive `nvs tmp clean`.

## Backlog

- After that item the goal may be closed: the driver's ledger reports only the *first* failing check,
  so the next acceptance run is what says whether stages 2-5 are all green
  (`docs/agent/loop-goal.toml:4319-4430`).
- `nvs-cli`'s fixture root wants the same `CARGO_MANIFEST_DIR` care a `[db.<name>]` block does — a
  relative path in a tree assembled in Rust resolves against whatever directory `cargo test` chose
  (`docs/agent/playbook.md`, *Writing a test case*).
