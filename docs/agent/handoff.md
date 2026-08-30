# Handoff

## State

**ADR 0020 § 1's tier 1 is complete on the argument side.** `Ctx::run_limit_handler`
(`crates/nvs-runtime/src/ctx.rs:1118`) takes the limit that stopped the request and hands the
handler § 1's `LimitReport`: one array whose `limit` key is the directive's own spelling. All three
entries pass their own — the helper boundary (`crates/nvs-runtime/src/abi.rs:321`, `Memory`), the
safepoint's CPU branch (`crates/nvs-runtime/src/ctx.rs:2341`) and its memory branch
(`crates/nvs-runtime/src/ctx.rs:2359`). `nvs_runtime::Limit` (`crates/nvs-runtime/src/ctx.rs:846`)
is the only home of *why an array and not a `Core` class* — the report is built in a crate that
holds no class descriptor — and ADR 0020 § 1's body now says the same thing, so the ADR and the
tree agree.

**The CPU half's time slice is still zero wide, on purpose.** Nothing reads `[limits] cpu_time`;
the flag is raised only by tests, so a handler entered from that branch runs straight-line work to
completion and is stopped again at its first back edge. The `nvs_safepoint` comment at
`crates/nvs-runtime/src/ctx.rs:2341` is that decision's only home.

The driver's failing check is closed. `a_malformed_file_leaves_the_previous_snapshot_serving_and_names_the_line`
was not on disk under any paraphrase — the previous handoff had closed the `[[app]]` check, not
this one — and is now `crates/nvs-config/tests/snapshot.rs:385`, asserting both halves: the tree is
refused before `Current::publish` is reached, and the refusal's span resolves to the offending line.

Still unfixed: `orient.py`'s `[context] modules` names `crates/nvs-host/src/budget.rs`, which never
existed — the accounting is `crates/nvs-runtime/src/budget.rs`. The pack warns every session.

## Next group

**The CPU-time limit, end to end.** File set: `crates/nvs-runtime/src/ctx.rs` (the limit cache at
`:1209`, the two configured-value readers at `:1268` and `:1226`, `set_config` at `:970`, the CPU
branch at `:2341`), `crates/nvs-config/src/tree.rs` (`[limits] cpu_time` at `:160`, the hard block's
at `:188`), and `crates/nvs-host/tests/limits.rs`.

- [ ] **`[limits] cpu_time` gets a reader** — ADR 0020 § 1 lists CPU time beside memory, and today
      nothing turns the directive into anything: `Ctx::refresh_limits`
      (`crates/nvs-runtime/src/ctx.rs:1209`) reads only the memory pair, beside
      `configured_memory_limit` (`crates/nvs-runtime/src/ctx.rs:1268`) and
      `configured_fatal_reserve` (`crates/nvs-runtime/src/ctx.rs:1226`), and
      `SafepointFlags::CPU_LIMIT` (`crates/nvs-runtime/src/ctx.rs:152`) is raised by tests alone.
      The decision to record is *what measures the time* — the deadline word this file already
      polls (`crates/nvs-runtime/src/ctx.rs:261`) is wall time, and § 1 names CPU time.
- [ ] **Then `fatal_reserve_time`, and only then** — § 1 names it beside `fatal_reserve_memory`
      (`crates/nvs-config/src/tree.rs:172`); until a reader exists the handler's time slice is zero
      wide, which `crates/nvs-runtime/src/ctx.rs:2341` states. Widening it is what makes a handler
      entered from the CPU branch able to loop, and `a_fatal_handler_runs_inside_its_reserved_slice`
      (`crates/nvs-host/tests/limits.rs:340`) is the shape its twin takes.
- [ ] **A `.nvst` case over the ladder** — every case that reaches `onLimit` today is a
      registration (`tests/conformance/core/fatal-on-limit-registers-a-handler-without-running-it.nvst:9`);
      the breach lives in `examples/limits.nvs:27` and is pinned by `loop-goal.toml` stage 4, not by
      a case. A handler declaring the report parameter and echoing its `limit` is the case that
      pins from source what `crates/nvs-host/tests/limits.rs:186` pins from Rust.

## Backlog

- `orient.py`'s `[context] modules` names a path that never existed — `docs/agent/loop-goal.toml`.
- Item 18's `Core\Secret::reveal()` is not in the registry — `crates/nvs-stdlib/src/secret.rs`.
- `Live::admit`'s same-class check asks the answer, not the argument — `crates/nvs-runtime/src/graph.rs`.
- Item 22's `Core\Script` members are unwritten — `crates/nvs-stdlib/src/script.rs`.
- § 1's other three limits have no `Limit` variant because they have no breach to report —
  `crates/nvs-runtime/src/ctx.rs:846`.
