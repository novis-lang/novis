# Handoff

## State

**Goal 7 — ADR 0131's temporary-directory sweep — has § 4 whole, on top of §§ 2, 3 and 5.** The walk,
both doors and the `[io]`/`[debug]` keys are on disk and green. What is left of the goal is § 3 on the
*request* path: two tests, with no product code known to be missing under them.

`sweep::orphans` (`crates/nvs-runtime/src/sweep.rs`) lists a root and answers the entries whose owner
is dead — sorted, deleting nothing — so both § 4 doors are that one walk plus what each does with the
list. `capability::temp_root` is public now and takes the configuration tree rather than a `&Ctx`,
because the doors have a tree and no context; its own doc is the home of every rule above, including
why a caller may only ever hand it the snapshot's tree.

`crate::serve::sweep_orphans` is the boot door: once, after the tree resolves and before anything
binds, and nothing it finds can refuse the start. `nvs tmp clean [--dry-run]` is the operator's door —
`crates/nvs-cli/src/tmp.rs` — with no force flag and an exit status about the configuration alone.

**Stage 3's check was filed `-p nvs-server` and could not have run there.** It is two checks now, in
both `docs/agent/loop-goal.toml` and `docs/agent/goals/7-temp-sweep.toml`: the boot half `-p nvs-cli`
(green), the post-response half `-p nvs-runtime` (open, and the group below). Stage 4's three tests
are green. Stages 2 and 5 are untouched.

**`orient.py` printed no map line for `crates/nvs-runtime/src/sweep.rs`** — this goal's own module —
because `[context] modules` reaches nvs-runtime only through `src/capability.rs`. Add
`nvs-runtime/src/sweep.rs`, `src/deferred.rs` and `src/ctx/hooks.rs` to that field; the next group
opens all three.

## Next group

**§ 3 on the request path: the two sweeps a worker owes.** One file set —
`crates/nvs-runtime/src/deferred.rs`, `crates/nvs-runtime/src/ctx/hooks.rs`,
`crates/nvs-runtime/src/sweep.rs`.

- [ ] **`a_requests_temporary_dirs_are_swept_after_its_after_response_work`** (0131 § 3, 0072 § 6) —
      the sweep runs *after* the deferred queue drains, asserted by ordering and not just by the
      directory being gone. `crates/nvs-runtime/src/ctx/hooks.rs:660` is the shape to copy: it is the
      same claim against the `onExit` queue, and it already builds the `callable` with no compiler in
      front of it. The queue is `crates/nvs-runtime/src/deferred.rs:184`, and the sweep it must
      precede is `crates/nvs-runtime/src/sweep.rs:83`.
- [ ] **`an_aborted_requests_dirs_are_swept_by_the_surviving_worker`** (0131 § 3, 0106) — a request
      that dies mid-flight still has its directories swept, because the worker survives it and the
      worker drops the context. Same fixture, one ending later:
      `crates/nvs-runtime/src/deferred.rs:184` and `crates/nvs-runtime/src/sweep.rs:83`.

## Backlog

- Stage 5's `a_finished_scripts_temporary_dir_is_gone_from_the_owned_root` (`-p nvs-cli`), over
  `examples/tempdir.nvs`, which is already on disk — `docs/agent/loop-goal.toml` stage 5.
- Stage 5's `kind = "exact"` run of `examples/tempdir.nvs` has not been seen to pass in a session —
  same file.
- `nvs tmp clean` takes no positional file list, unlike `config check` and `queue migrate`. ADR 0131
  § 4 asks for none; if an operator ever needs one, it is `crates/nvs-cli/src/tmp.rs`.
