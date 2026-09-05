# Handoff

## State

**Goal 6, M7 — the driver's `native examples/pool.nvs [1 floor]` failure is closed, and it was not
that fixture.** A per-stack thread-local was restored across a coroutine switch:
`nvs_runtime::ctx::CurrentCtx` saves the thread's two context words on install and writes them back
on drop, so a task that installed while another was parked saved *that* task's words and wrote them
back after its `Ctx` had been dropped — a freed `Ctx` and a freed `LiveList` on the thread, and the
next object allocated on the core linking itself onto a list that was gone.

`nvs_host`'s `yield_on` now carries the pair off the stack and back beside `HelperFrame`'s count and
`RUNNING`, through the new `nvs_runtime::CurrentStack`; that type's doc comment in
`crates/nvs-runtime/src/ctx/current.rs` is the home of why. `crates/nvs-host/src/scheduler.rs`'s
`a_task_that_parks_leaves_no_context_on_the_thread_for_the_next_one` is the interleaving that
produced it and fails without the carry.

**What triggered it is still on disk and is now a Backlog item**, not a blocker: the queue suite
leaves claimable rows in `novis_test`, so any `nvs run` in this checkout may print two unrelated
stderr lines. Stage 7's `nvs service` check stays green; the driver's next failure is the
`nvs-server (hot reload)` check at `docs/agent/loop-goal.toml:3915`, which is the group below.

## Next group

**Stage 7's `nvs-server (hot reload)` check. The file set is `docs/agent/loop-goal.toml`,
`crates/nvs-cli/src/script.rs` and `crates/nvs-config/src/cache.rs`.** All three names are misfiled
`-p nvs-server`: two are the CLI cache's and one is `nvs_config::cache`'s.

- [ ] **Refile the check and write `revalidation_is_lazy_and_rate_capped`** — split
      `docs/agent/loop-goal.toml:3915` so the two cache names run `-p nvs-cli`, and assert both
      halves of ADR 0017 § *Decision* step 1 over `crates/nvs-cli/src/script.rs:235`:
      `validate = "never"` never `stat`s, and two resolves inside one `revalidate_freq` window make
      one check while one past it makes two. Count the checks by editing the file between resolves
      and reading which unit comes back — `crates/nvs-cli/src/script.rs:397` is the only place a
      syscall happens, and `revalidating()` at `crates/nvs-cli/src/script.rs:552` is the fixture.
- [ ] **`a_swap_never_blocks_a_request_serving_core`** — ADR 0017 § *Decision*'s paragraph after the
      five steps, over `crates/nvs-cli/src/script.rs:235`: a resolve that finds a newer source
      publishes the new unit without any in-flight resolve waiting on it.
- [ ] **`the_validate_default_is_selected_by_the_run_mode`** — and **it is not `nvs_config::mode`'s**,
      which an earlier handoff guessed. `crates/nvs-config/src/mode.rs:8` says outright that
      § 3a's three startup rows are deliberately *not* in that module's table. The default lives at
      `crates/nvs-config/src/cache.rs:204` (`Validate`'s `#[default]`, whose doc already cites
      ADR 0091) and is read at `crates/nvs-config/src/cache.rs:262` (`Revalidation::of`), which
      today reads `[opcache]` and does **not** take a mode — so check whether the mode→`validate`
      link exists before writing the test.

## Backlog

- The queue suite leaves claimable jobs in the shared `novis_test`: `clear` at
  `crates/nvs-stdlib/tests/queue.rs:395` runs at the head of a case and never at the end, and the
  roster in `crates/nvs-cli/src/worker.rs:239` claims from every queue. Nineteen call sites, each
  ending with a different connection state — the cheap fix is not obvious, and the playbook bullet
  above is what stops the next session re-deriving the diagnosis.
- Registration itself — `install`/`uninstall`/`start`/`stop`/`status`/`run`, the SCM call and the
  systemd write. `crates/nvs-cli/src/service.rs`'s module doc § *What is on disk, and what is not*.
- `nvs ctl` has no client and § 3's `ctl config` is not an operation —
  `crates/nvs-server/src/control.rs`'s module doc.
- Nothing accepts on the control endpoint: no `hyper` connection is served over it and `nvs serve`
  creates none. Same module doc.
- `--fault-inject` is matched by word in `service.rs`; if `nvs run` ever gains a second hook the
  allowlist there needs the same treatment. ADR 0093 § 2, row 2.
- `[server] listen`'s privileged-port test in `service.rs::describe_host` reads the merged table
  rather than `nvs_config::server`'s typed value. ADR 0097 § 5.
