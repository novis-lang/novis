# Handoff

## State

**Goal 6, M7 — ADR 0083 § 7's four `-p nvs-server` names are green and the check is split.**
`every_connection_bound_is_finite_with_nothing_configured` (`crates/nvs-server/src/bounds.rs:213`), the
over-budget close, the contained panic and `reload_and_shutdown_close_every_connection_after_the_drain`
(the last three in `crates/nvs-server/src/serve.rs`'s test module) are the whole of that check now.

**The two ADR 0017 names moved to a second `[[check]]`, `-p nvs-cli`, and both are blocked on a
mechanism rather than on their filing.** `crates/nvs-cli/src/script.rs`'s `Compiler` is the tree's only
in-memory unit table, and it compiles once per written path and never revalidates: `[opcache] validate`
and `revalidate_freq` deserialize in `nvs_config::tree` with nothing reading them, and
`nvs_config::cache::UnitKey` — ADR 0017's `{ path, content_hash, env_hash }` — has no caller outside
`nvs-config`'s own tests. So § 7's second bullet is unfalsifiable against this tree: every holder keeps
its `Program` across an edit when nothing can observe one. `script.rs`'s module doc now carries that as
a `# Known gap` section, which is its one home; the new check's comment carries the filing argument.

**Stage 7's `nvs-server (hot reload)` check is misfiled the same way** —
`a_swap_never_blocks_a_request_serving_core` and `revalidation_is_lazy_and_rate_capped` are the same
cache's, and `the_validate_default_is_selected_by_the_run_mode` is `nvs_config::mode`'s ADR 0091 § 3a
row. Left alone: which crate each moves to is decided by where the swap lands, which is the group below.

**`[context]` gained `0083 §7` and `crates/nvs-cli/src/script.rs` and lost `0083 §§ 1-4`** — the driver
stops at the first failing check, so every 6b check above § 7's is passing and those sections are spent.
`docs/agent/goals/6-server.toml` is back in sync with the live file, which it had not been since
2026-09-04.

## Next group

**ADR 0017's swap, then the two names it unblocks. The file set is `crates/nvs-cli/src/script.rs`,
`crates/nvs-cli/src/serve.rs` and `crates/nvs-config/src/cache.rs`.** The first slice is the whole of
it; the two tests are cheap once it lands, which is why the group is three.

- [ ] **Give `Compiler` ADR 0017 § *Decision*'s five steps** — a `PathEntry { content_hash,
      last_checked }` in front of a unit table keyed by `nvs_config::cache::UnitKey`
      (`crates/nvs-config/src/cache.rs:143`, whose `new` takes the content digest so the source is read
      once). `crates/nvs-cli/src/script.rs:105` is the `Compiler` and
      `crates/nvs-cli/src/script.rs:118` is the `compiled` that answers from the map forever today.
      **Single-core collapses most of the ADR**: that cache is a `RefCell<HashMap>` reached from one
      coroutine, so there is no compile pool, no `Compiling`/`Ready` broadcast and no single-flight —
      steps 1-5 are a `stat`, a lookup, a recompile and a pointer write, all synchronous, and step 4's
      "only if a fresher revalidation has not won" is unreachable rather than wrong. Say so in the doc
      rather than implementing the machinery. `[opcache] validate` and `revalidate_freq` are at
      `crates/nvs-config/src/tree.rs:910`; `Compiler` is `Default`-constructed at
      `crates/nvs-cli/src/serve.rs:263`, `crates/nvs-cli/src/main.rs:1241` and
      `crates/nvs-cli/src/runner.rs:351`, so reading them means a constructor and three call sites.
      Leave the mode-selected `validate` default (ADR 0017's own § *Decision*, last paragraph but two)
      to stage 7 — it is `nvs_config::mode`'s row, not this cache's.
- [ ] **`an_open_connection_keeps_its_compiled_unit_across_an_edit`** — ADR 0083 § 7's second bullet,
      first half, as a `-p nvs-cli` test. `crates/nvs-cli/src/script.rs:219`'s `granting_ctx` is the
      context a fixture that spawns needs, and `crates/nvs-cli/src/serve.rs:294` is the per-request
      resolve that must go on answering the old unit while the file underneath has changed.
- [ ] **`a_connection_opened_after_the_swap_runs_the_new_unit`** — the other half, over the same
      fixture: resolve once, edit, resolve again, and assert the second answer is the new code.
      `crates/nvs-cli/src/script.rs:118` is the one seam both assertions read.

## Backlog

- Stage 7's `nvs-server (hot reload)` check needs the same split its comment now predicts —
  `docs/agent/loop-goal.toml`, the block at `stage = "7 operator surface"`.
- `crates/nvs-cli/src/serve.rs:570` claims a scheduled entry "picks up an edited script at the next
  fire"; that is false until the swap lands, and it is one sentence to fix when it does.
- A connection already parked on a read does not see a drain until that read ends —
  `crates/nvs-server/src/socket.rs`'s `receive` states it; the same wake seam `nvs_runtime::Ctx::deliver`
  wants.
- `[context] modules` still does not print `crates/nvs-host/src/scheduler.rs`; the drain item that
  wanted it is landed, so add it only when a session needs cancellation's safepoint rule again.
