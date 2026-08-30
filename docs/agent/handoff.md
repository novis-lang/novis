# Handoff

## State

**The driver's failing check is closed.** ADR 0078 § 4's `env_hash` and both cache keys are
`crates/nvs-config/src/cache.rs` — one `EnvHash` over the sorted `[[extension]]` pins, the on-disk
`artifact_key(source, env)` and the in-memory `UnitKey { path, content_hash, env }`, whose fields are
private so the pre-§ 4 key cannot be spelled. Both keys live in one crate on purpose; that module doc
is the only home of why, and of the known gap that `compiler_version_hash` is the release version, so
two builds of one version share it.

**`[limits] cpu_time` has a reader.** `Ctx::cpu_limit` (`crates/nvs-runtime/src/ctx.rs:1281`) is
nanoseconds, `0` for no cap, resolved by `refresh_limits` like the memory pair. The decision the item
asked for is recorded in the field doc at `crates/nvs-runtime/src/ctx.rs:342`: **the request thread's
own CPU clock, sampled by whatever timer watches the request, and never the wall clock `deadline` is
written from** — per-thread because a thread-per-core host runs many requests in one process, and
off-thread because `nvs-runtime` has no platform dependency to read a clock with. Nothing samples one
yet, so `SafepointFlags::CPU_LIMIT` is still raised by tests alone and the CPU branch's time slice is
still zero wide.

Still unfixed: `orient.py`'s `[context] modules` names `crates/nvs-host/src/budget.rs`, which never
existed — the accounting is `crates/nvs-runtime/src/budget.rs`. The pack warns every session.

## Next group

**The rest of the CPU-time limit.** File set: `crates/nvs-runtime/src/ctx.rs` (the limit cache at
`:1236`, the two readers at `:1258` and `:1291`, `run_limit_handler` at `:1145`, the CPU branch
inside `nvs_safepoint` at `:2411`) and `crates/nvs-runtime/tests/configured_limits.rs`, which is the
reader's own suite and already builds a snapshot from a TOML string.

- [ ] **`fatal_reserve_time` gets a reader** — ADR 0020 § 1 names it beside `fatal_reserve_memory`,
      and `reserve_within` (`crates/nvs-runtime/src/ctx.rs:1318`) is the shape to copy: a default, a
      clamp against the ceiling, and the number stated here rather than in the ADR. It carves out of
      `Ctx::cpu_limit` (`crates/nvs-runtime/src/ctx.rs:1281`) exactly as the memory one carves out of
      `memory_limit`, so `refresh_limits` (`crates/nvs-runtime/src/ctx.rs:1236`) sets both in one
      pass. A case joins `crates/nvs-runtime/tests/configured_limits.rs:41`.
- [ ] **The CPU branch's slice stops being zero wide** — `nvs_safepoint`
      (`crates/nvs-runtime/src/ctx.rs:2411`) enters the handler under the flag that stopped the
      request, so the handler is stopped again at its first back edge. With a reserve to spend, the
      branch clears `CPU_LIMIT` for the handler's own run the way the memory branch spends its
      reserve, and `run_limit_handler` (`crates/nvs-runtime/src/ctx.rs:1145`) owns the zero-retry
      rule that keeps that from being a second chance at the request.
- [ ] **A `.nvst` case over the whole ladder** — every case that reaches `onLimit` today is a Rust
      one (`crates/nvs-host/tests/limits.rs:263`), so nothing pins what a *program* sees: a handler
      registered, a limit breached, the report's `limit` key read, and no `catch` entered.

## Backlog

- The timer that raises `SafepointFlags::CPU_LIMIT` from a real clock — `crates/nvs-host`, per
  `crates/nvs-runtime/src/ctx.rs:342`'s field doc.
- `compiler_version_hash` is the release version, so a rebuilt compiler reuses artifacts —
  `crates/nvs-config/src/cache.rs`'s module doc.
- Nothing constructs a `UnitKey` or an `artifact_key` yet; the caches themselves are ADRs 0017 and
  0042, unstarted.
- Item 18's `Core\Secret::reveal()` is not in the registry — `docs/implementation-plan.md`.
- `Live::admit`'s same-class check is asked of the answer, not the argument —
  `crates/nvs-runtime/src/graph.rs` § *Known gaps*.
- Item 22's `Core\Script` members are unwritten — `crates/nvs-stdlib/src/script.rs`.
