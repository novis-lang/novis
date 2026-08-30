# Handoff

## State

**ADR 0020 § 1's tier 1 is reached by both limits that can stop a running program.** The memory
breach reaches it at the helper boundary (`crates/nvs-runtime/src/abi.rs:321`) and at the safepoint
poll (`crates/nvs-runtime/src/ctx.rs:2308`); the CPU-time flag now reaches it at the same poll
(`ctx.rs:2299`), one `run_limit_handler()` above its `set_pending`, so a request stopped for time
gets the same last word as one stopped for memory. `Ctx::run_limit_handler` (`ctx.rs:1075`) is
still the only home of the zero-retry rule and of the reserve's lifetime.

**The CPU half's time slice is zero wide, on purpose.** § 1 names `fatal_reserve_time` beside the
memory one, and nothing reads `[limits] cpu_time` yet — the flag is raised only by tests today — so
a handler entered from the CPU branch runs straight-line work and `Core` calls to completion and is
stopped again at its own first back edge. The `nvs_safepoint` comment at `ctx.rs:2299` is that
decision's only home; adding the directive before something reads it would be a row with no reader.

The driver's failing check is closed. All three names under `nvs-config (the app block)` were on
disk as paraphrases in `crates/nvs-config/tests/app.rs`; two were renames, and the third
(`..`-or-symlink) is now one case asserting both halves, which is how M6's *Verify* states it.

Still unfixed: `orient.py`'s `[context] modules` names `crates/nvs-host/src/budget.rs`, which never
existed — the accounting is `crates/nvs-runtime/src/budget.rs`. The pack warns every session.

## Next group

**The rest of ADR 0020 § 1's ladder.** File set: `crates/nvs-runtime/src/ctx.rs`
(`run_limit_handler` at :1075, the two poll branches at :2299 and :2308),
`crates/nvs-runtime/src/closure.rs` (`call_closure` at :132), `crates/nvs-stdlib/src/fatal.rs` (the
row at :38, the card at :52, the body at :84), and `crates/nvs-host/tests/limits.rs`.

- [ ] **`LimitReport` is the argument the handler is handed** — ADR 0020 § 1
      (`docs/adr/0020-error-escalation-ladder.md:66`) spells the parameter `closure(LimitReport)`.
      `crates/nvs-runtime/src/ctx.rs:1075` calls with `&[]` today, and
      `crates/nvs-runtime/src/closure.rs:132` refuses a handler that declares one, so a program
      following the ADR's own signature fails. What the report *is* — a `Core` class, a shape, or a
      map — is a decision this goal pre-authorizes; whichever it is, it names which limit stopped
      the request, so both branches at `crates/nvs-runtime/src/ctx.rs:2299` and
      `crates/nvs-runtime/src/ctx.rs:2308` must pass their own. The
      card at `crates/nvs-stdlib/src/fatal.rs:52` states the parameter and has to move with it.
- [ ] **`[limits] cpu_time` gets a reader, and only then `fatal_reserve_time`** —
      `crates/nvs-runtime/src/ctx.rs:1597` (`request_safepoint`) is the only way `CPU_LIMIT` is
      raised and nothing outside tests calls it, so the branch at
      `crates/nvs-runtime/src/ctx.rs:2299` is unreachable in a
      real run. The reserve's time half follows whatever raises it, not before.
- [ ] **A `.nvst` case over the whole ladder** — the previous handoff's premise was wrong: three
      cases under `tests/conformance/core/` already register a handler
      (`tests/conformance/core/fatal-on-limit-registers-a-handler-without-running-it.nvst:12`).
      What none of them does is *breach*, and a breaching script exits non-zero
      (`examples/limits.nvs:6`), which is why `examples/fatal.nvs` sits on the goal's own skip list.
      Read `crates/nvs-test`'s module doc for whether a case can pin a non-zero exit before writing
      one; if it cannot, that is the finding, and the `nvs-host` tests stay the pin.

## Backlog

- `Core\Secret::reveal()` is not in the registry — item 18, `crates/nvs-stdlib/src/registry.rs`.
- `Live::admit`'s same-class check asks the answer, not the argument — `crates/nvs-runtime/src/graph.rs`.
- Item 22's `Core\Script` members are unwritten — `crates/nvs-stdlib/src/script.rs`.
- `[context] modules` in `docs/agent/loop-goal.toml` names a `budget.rs` that never existed.
