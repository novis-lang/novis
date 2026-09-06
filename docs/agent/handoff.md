# Handoff

## State

**Goal 7 — ADR 0131's temporary-directory sweep — has §§ 2-3 on disk and working end to end.**
`capability::temp_dir` creates under the root Novis owns and now **records** every directory it
hands back (`Ctx::track_temporary_dir`); `crate::sweep::at_script_end` deletes the recorded list and
is called from `Ctx::drop` and nowhere else — that module's doc owns why a context's teardown *is*
§ 3's "after the last user code" for every ending at once, and why the open files are closed
immediately before it. `examples/tempdir.nvs` runs and leaves the owned root empty.

**The acceptance failure that opened this session is closed**: `examples/tempdir.nvs` exists, prints
the frozen line, and `nvs.toml` carries its `[[app]]` grant beside `examples/files.nvs`'s.

**Nothing of §§ 4-5 exists** — no orphan sweep, no `nvs tmp clean`, no `[debug] keep_temporary`. The
design is settled and is not reopened:
[ADR 0131](../adr/0131-a-temporary-directory-dies-with-its-script-and-the-sweep-never-throws.md) is
the specification.

**Stage 2's `-p nvs-host` check is filed in the wrong crate and the first item below fixes it.** The
sweep runs from `Ctx::drop`, so nvs-host neither calls it nor can observe it doing anything an
`nvs-runtime` test cannot; and `the_sweep_runs_after_the_on_exit_queue` needs an exit-hook closure,
which is `allocation_policy.rs`'s `closure_of` shape and no cheaper in nvs-host than beside the queue
itself.

## Next group

**The rest of § 3's evidence, then § 5's key** — one file set: `crates/nvs-runtime/src/sweep.rs`,
`crates/nvs-runtime/src/ctx/hooks.rs`, `crates/nvs-config/src/tree.rs`,
`crates/nvs-config/src/directive.rs`, `docs/agent/loop-goal.toml`.

- [ ] **The two remaining stage-2 tests, beside the code they are about** (0131 § 3) —
      `the_sweep_runs_after_the_on_exit_queue` (a hook that observes its own temporary directory
      still standing, `crates/nvs-runtime/src/ctx/hooks.rs:229`) and
      `a_refused_deletion_logs_and_never_throws` (the log half of
      `crates/nvs-runtime/src/sweep.rs:59`; `refusals` at `crates/nvs-runtime/src/sweep.rs:111` is
      already asserted without a sink). Re-file the check at `docs/agent/loop-goal.toml:4326` from
      `-p nvs-host` to `-p nvs-runtime` in the same slice, with a comment saying why.
- [ ] **`[debug] keep_temporary`, the operator's only escape hatch** (0131 § 5) — the sweep logs each
      path it would have deleted instead of deleting it. The block is
      `crates/nvs-config/src/tree.rs:340`, the reloadable row goes beside
      `crates/nvs-config/src/directive.rs:160`'s `io.temp_root`, and the branch is at
      `crates/nvs-runtime/src/sweep.rs:59`. Stage 5's check names it `-p nvs-host`
      (`keep_temporary_logs_each_kept_path_and_deletes_none`) and it has the first item's problem.
- [ ] **§ 4's owner-liveness predicate, as a function over a path and nothing else** (0131 § 4) —
      the pid is already in the entry's name (`crates/nvs-runtime/src/capability.rs:780`), so this is
      parse-then-ask and it belongs beside the sweep at `crates/nvs-runtime/src/sweep.rs:111`. The
      goal's standing decisions pre-authorise "ambiguity resolves toward skip", recorded in that
      module's doc.

## Backlog

- Stage 3's two server sweeps — `docs/agent/loop-goal.toml:4342`. `Ctx::drop` may already satisfy
  both; the slice is the tests plus whatever the request path does not reach.
- `nvs tmp clean` (0131 § 4) — `crates/nvs-cli/src/main.rs`'s subcommand table.
- The `[context] modules` manifest has no `nvs-runtime/src/sweep.rs` pattern yet; it printed because
  the crate matched, but check it after any narrowing.
