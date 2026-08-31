# Handoff

## State

**ADR 0020 § 6's tier-4 floor is built, and it is one record and one serialiser reached twice.**
`nvs_runtime::floor` builds an `nvs_render::Record` at `Level::Error` from an uncaught `Thrown` —
message, a `class` field and, where there are frames, a `backtrace` field — and renders it with the
same `nvs_render::json::line` `Core\Log::write` calls. That module's own doc comment is the home of
why the envelope's `ts`/`request_id`/`trace_id` stay unset (parity with `nvs_stdlib::log`'s
`record`: a floor filling a field its ordinary-code twin does not is the schema divergence § 6
forbids) and of why `[log] format` is not read yet.

**Two callers, both on the diagnostic channel:** `nvs run`'s uncaught arm
(`crates/nvs-cli/src/main.rs:967`) and after-response work that threw or timed out
(`crates/nvs-runtime/src/deferred.rs`, which adds its own `origin` field). The `FATAL` arm is
untouched — tier 3 and above, and seven `.nvst` cases pin its wording.

**`nvs-render` is a leaf now.** ADR 0087's bidi predicate **moved** from `nvs_syntax::bidi` to
`nvs_render::bidi` and `nvs-syntax` reads it from below, so `nvs-runtime → nvs-render` closes no
cycle. That was the price the item named and it is paid; `crates/nvs-render/src/lib.rs`'s
§ *Where this sits* is rewritten as the home of the result. `Thrown::class_name` moved down the same
way, out of `nvs_host::isolate`, because the floor asks the same question of the same value.

**The acceptance check still fails on its third line, and only tiers 3 and 4 are between it and
green.** `examples/logging.nvs`'s stdout is unchanged by this slice — the child's floor record goes
to *stderr* — so `handler ran` is still owed by tier 3. Both fixtures exist:
`examples/logging/handler.nvs` prints that line and `examples/logging/throws.nvs` is the child.

## Next group

**Tier 3 — ADR 0020 § 3's handler isolate — over `crates/nvs-cli/src/main.rs`,
`crates/nvs-host/src/isolate.rs`, `crates/nvs-config/src/tree.rs` and `nvs.toml`.**

- [ ] **The floor reads `[log] handler` and spawns it as an isolate before reporting** — ADR 0020
      § 3. The key is declared and nothing reads it: `crates/nvs-config/src/tree.rs:330` is the
      `Log` struct, `crates/nvs-cli/src/main.rs:967` is the uncaught arm that must try tier 3
      first, and `crates/nvs-host/src/isolate.rs:111` (`Isolate::new(program, args, output)`) and
      `:133` (`run`) are the spawn ADR 0006 already defines. `nvs.toml` has no `[log]` block at
      all yet; it needs one naming `examples/logging/handler.nvs` for the
      `examples/logging/throws.nvs` app. **Zero retries**: a handler that throws or times out
      drops straight to `crates/nvs-runtime/src/floor.rs:60`'s `uncaught`, which is already there.
- [ ] **The isolate is charged to an engine-owned reserve, not to the failing request** — ADR 0020
      § 3's one deliberate exception to ADR 0006, sized once per core at boot from
      `crates/nvs-config/src/tree.rs:335`'s `handler_reserve_memory` and `:337`'s
      `handler_reserve_time`. This is the part that makes tier 3 still run for a request already at
      its ceiling, so a case that only proves the happy path has not proved the section.
- [ ] **`examples/logging.nvs` green on its three frozen lines** — `docs/agent/loop-goal.toml:2455`.
      The first two already print; the third is `handler ran` on stdout, which arrives through the
      child's `output: 'inherit'` once the two above land. This closes the driver's standing
      acceptance failure.

## Backlog

- `Core\Fatal\ErrorReport` — ADR 0020 § 3's one argument to the handler, through
  `Core\Script::args()` (`crates/nvs-stdlib/src/script.rs:316`). Not built; tier 3 can land with a
  record-shaped array first and gain the class with `Core\Fatal` — `docs/plan/m8.md`'s verify list.
- `[log] format` is read by nobody. ADR 0091 § 3 (`crates/nvs-config/src/mode.rs:78`) makes it
  `text` in development and `json` in production; the floor and `Core\Log::write` both hardcode
  JSON. `nvs_render::plain::render` is the other rendering, already written.
- `[log] target` — `stderr` / `file:<path>` / `syslog`, ADR 0020 § 4. Also read by nobody; the floor
  writes `Ctx`'s diagnostic channel unconditionally.
- ADR 0076 § 6's `trace_id`/`span_id` in the envelope, when a trace is active. `Ctx` already holds a
  `TraceContext`; neither log caller reads it, and both must gain it together.
- `orient.py` printed no ADR 0020 sections. Its `[context] adrs` needs `0020:3`, `0020:5` and
  `0020:6` — this session's item cited §§ 5-6 by number and had to slice all three by hand.
