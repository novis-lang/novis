# Handoff

## State

**ADR 0020 § 1's memory limit is enforced and its request-facing edges are pinned.** `budget` counts
in every build, `Ctx` holds the ceiling and refreshes it from the overlay, a breach is a `FATAL`, and
two conformance cases now hold the ceiling from a program: `config-set-memory-above-the-default-…`
raises 16M to 192M and holds 64 MiB against it, `config-set-memory-above-the-hard-ceiling-…` pins 32M
accepted and 33M refused and then proves the refusal never reached the runtime by breaching at 64 MiB.

**The driver's failing check passes.** It named
`an_unknown_key_is_refused_naming_the_block_that_has_no_such_directive`;
`crates/nvs-config/tests/tree.rs:94` held the same assertions under a shorter name and now carries the
name the check asks for. Nothing about the refusal changed.

Still true and still unfixed: `orient.py`'s `[context] modules` names `crates/nvs-host/src/budget.rs`,
which never existed — the accounting is `crates/nvs-runtime/src/budget.rs`. The pack prints its own
warning about it every session.

## Next group

**The rest of ADR 0020 § 1's ladder** — the breach is produced and printed, and nothing between the
two is reachable from a program. File set: a new `crates/nvs-stdlib/src/fatal.rs` beside
`crates/nvs-stdlib/src/config.rs`, `crates/nvs-stdlib/src/registry.rs:984` (`CLASSES`),
`crates/nvs-runtime/src/ctx.rs:944` (`memory_breach`, and `memory_limit`'s fields above it at :920)
and `crates/nvs-cli/src/main.rs:828`, which is where a `FATAL` is printed today.

- [ ] **`Core\Fatal::onLimit` exists and holds a handler** — ADR 0020 § 1
      (`docs/adr/0020-error-escalation-ladder.md:66`; read that section, the class is specified there
      and nothing of it is on disk — there is no `Core\Fatal` module and no `LimitReport`). The five
      edits are conventions.md § *A `Core` member*; the closure lives on `Ctx` beside
      `memory_limit` (`crates/nvs-runtime/src/ctx.rs:920`), because the breach is asked there.
- [ ] **The breach reaches that handler before the ladder prints** — the `Fault` is built at
      `crates/nvs-runtime/src/ctx.rs:944` and raised at `crates/nvs-runtime/src/abi.rs:305` (the helper
      boundary) and `crates/nvs-runtime/src/ctx.rs:2064` (the safepoint); it is rendered at
      `crates/nvs-cli/src/main.rs:828`. § 1's zero-retry, reserved-budget rule is what decides where
      between those the call sits. A `.nvst` case with `--EXPECT--` and `--EXPECTF-ERROR--` pins both
      halves — the two `config-set-memory-*` cases are the shape.
- [ ] **The CPU-time limit reaches the same ladder** — ADR 0020 § 1. `cpu_time` sits unread beside
      `memory` in `crates/nvs-config/src/tree.rs:156`; `Ctx`'s ceiling pair and `refresh_limits`
      (`crates/nvs-runtime/src/ctx.rs:955`) are the shape it copies.

## Backlog

- `[context] modules` in `docs/agent/loop-goal.toml` should name `crates/nvs-runtime/src/budget.rs`.
- Item 18: `Core\Secret::reveal()` is not in the registry — `docs/implementation-plan.md` § *Open now*.
- `Live::admit`'s same-class check is asked of the answer, not the argument —
  `crates/nvs-runtime/src/graph.rs` § *Known gaps*.
- Item 22: `Core\Script`'s members are unwritten — `crates/nvs-stdlib/src/script.rs`.
- `python tools/gaps.py`, `holes.py` and `check-migration.py --report` are the standing worklists.
