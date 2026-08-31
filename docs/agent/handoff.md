# Handoff

## State

**ADR 0020's ladder now has tiers 3 and 4, and tier 3 is an ordinary isolate.**
`nvs_host::ladder::escalate` reads `[log] handler` off the context's own snapshot, resolves it
through ADR 0118 § 2's spawn door, and runs it as `Isolate::new(program, args, Output::Inherit)`.
It answers `true` when the handler completed, and only a `false` lets `nvs_runtime::floor::report`
write the tier-4 line — so the floor is the floor *beneath* tier 3 and never a second copy beside
it. That module's own doc comment is the home of the zero-retries guard (a thread-local, not a `Ctx`
field), of why the spawn lives in `nvs-host` while the argument lives in `nvs_runtime::floor`, and
of the reserve gap below.

**Two callers, both "a program's throw reached the top".** `nvs run`'s root task
(`crates/nvs-cli/src/main.rs:891`, inside the task body rather than beside the exit code, because
an isolate wants the scheduler, reactor and resolver the run installs and takes down) and an
isolate's own `crates/nvs-host/src/isolate.rs:399`. The isolate caller deliberately does **not**
fall through to tier 4: ADR 0006 already hands that failure to the parent as a value, and
`finish`'s comment is the home of that reading.

**The handler's argument is a keyed `array<string, string>`**, built by
`nvs_runtime::floor::report_argument` from the same `Record` tier 4 would have written — `level`,
`message`, and one key per envelope field. Its doc comment owns why an array rather than a
`Core\Fatal\ErrorReport` instance, and why a non-string node is skipped rather than stringified.

**`[log] handler` is per application and `System`-class.** `nvs_config::tree::App` gained a `log`
field, so `[app.log] handler` folds onto the global block through `snapshot`'s existing per-app
merge; `directive.rs` carries `log.handler` as a more specific `System` row over its `Runtime`
parent, because § 3 says so and because the value names a file to run. `nvs.toml`'s block is keyed
on `examples/logging.nvs` — see the playbook bullet for why not on the child that throws.

**The acceptance check on `examples/uncaught.nvs` was stale, not broken.** Its `want` still read
`Uncaught Exception: unhandled` from before the floor rendered JSON; ADR 0020 § 6 and ADR 0092 § 3
make JSON Lines the log target's default rendering, so the expectation was rewritten against the
record. `examples/logging.nvs` prints its three frozen lines.

## Next group

**ADR 0020 § 3's engine-owned reserve, over `crates/nvs-host/src/ladder.rs`,
`crates/nvs-runtime/src/ctx.rs`, `crates/nvs-config/src/directive.rs`, `crates/nvs-config/src/tree.rs`
and `crates/nvs-stdlib/tests/` — the same files this group touched plus the tests that pin it.**

- [ ] **The tier-3 isolate is charged to an engine-owned reserve, not to the failing request** —
      ADR 0020 § 3's one deliberate exception to ADR 0006. Today `crates/nvs-host/src/ladder.rs:100`
      hands `Isolate::run` the failing context, so `crates/nvs-runtime/src/ctx.rs:2192`
      (`Ctx::isolate`) clones its budget and its `deadline` — a request at its ceiling has nothing
      to lend and the handler dies with it. The reserve is `[log] handler_reserve_memory` and
      `handler_reserve_time`, already fields on `crates/nvs-config/src/tree.rs:337`; they need the
      two `System`/`Reload` rows beside `log.handler` at `crates/nvs-config/src/directive.rs:118`,
      and `Ctx::isolate` needs a sibling that starts from the reserve rather than from the parent.
      `crates/nvs-runtime/src/ctx.rs:1881` (`configured_fatal_reserve`) is the reader to copy.
- [ ] **`the_configured_handler_script_runs_charged_to_the_engines_own_reserve` and
      `the_handler_still_fires_when_the_reporting_request_is_at_its_memory_ceiling`** —
      `docs/agent/loop-goal.toml:2436` names both as `-p nvs-stdlib` cases and neither exists.
      Check they can be hosted there at all before writing them: the ladder is `nvs-host`'s and
      `nvs-stdlib` does not depend on it, so the honest home may be `crates/nvs-host/tests/limits.rs`
      with the goal's `args` corrected — the playbook's *a loop-goal.toml check can name a test in a
      crate that cannot host it* bullet is this exact shape.
- [ ] **`application_code_and_the_engine_floor_produce_schema_identical_records`** — the same
      `cargo-named` block. `crates/nvs-stdlib/src/log.rs`'s `record` and
      `crates/nvs-runtime/src/floor.rs:58` (`uncaught`) are the two builders, and both render through
      `nvs_render::json::line`, so the case is one assertion over two envelopes.

## Backlog

- ADR 0020 § 2's `Core\Fatal::onUncaughtThrow` receiving the real `Throwable` — `docs/plan/m8.md`.
- `[log] format = "text"` is still unread at run time — `crates/nvs-runtime/src/floor.rs`'s module doc.
- `[log] target` (`file:`/`syslog`) is unread; tier 4 writes to the diagnostic channel only — same doc.
- ADR 0106's amendment: the floor rotates and rate-limits itself — `docs/agent/loop-goal.toml:2441`.
- Stage 8, introspection: `Core\Reflect`, `Core\Ast`, `Core\Decimal` — `docs/plan/m8.md`.
- A `FATAL` still never reaches tier 3 — `crates/nvs-cli/src/main.rs`'s `Err(_)` arm says so.
