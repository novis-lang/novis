# Handoff

## State

**ADR 0020 § 3's engine-owned reserve is on disk.** `Ctx::handler_isolate`
(`crates/nvs-runtime/src/ctx.rs`) is `Ctx::isolate` with three things parted from it: its own
deadline word, its own two ceilings from `[log] handler_reserve_memory` / `handler_reserve_time`,
and a fresh script depth. That constructor's doc comment is the one home of why each of the three,
and of why the reserve is a ceiling rather than a pre-allocation; `Ctx::DEFAULT_HANDLER_RESERVE_MEMORY`
and `..._TIME` own the two numbers (16 MiB, 5 s) where the configuration states neither. Both keys
are `System`/`Reload` rows in `nvs_config::directive`, beside `log.handler` and for their own
reason.

**The ladder asks for it and nothing else can.** `Isolate::charged_to_the_engine_reserve` is a
builder, not an option on `Isolate::new`, because § 3 says the exception is not a precedent — the
`spawn script` seam never calls it. `Isolate::start`'s `max_script_depth` refusal is now asked of
the tree only: a chain at that ceiling is one of the failures tier 3 exists to report.

**Stage 7 owes one item**, the schema-identity test in the backlog below. **The driver's failing
acceptance check is stage 8 and not a regression**: `examples/reflect.nvs` wants
`Core\Reflect::forObject` and there is no `crates/nvs-stdlib/src/reflect.rs` at all — see the new
playbook bullet.

## Next group

**Stage 8's opening — `Core\Reflect`, over `crates/nvs-stdlib/src/registry.rs`, a new
`crates/nvs-stdlib/src/reflect.rs`, `crates/nvs-stdlib/src/lib.rs`,
`crates/nvs-stdlib/src/instance.rs` and `tests/conformance/core/`. Stage 8's own rules and its
frozen five lines are `docs/agent/loop-goal.toml:2463`; the fixture is `examples/reflect.nvs:25`.**

- [ ] **`Core\Reflect::forObject`, and the described class's `name` and `properties()`** — ADR 0019's
      first rule. The five edits of *A `Core` member* over `crates/nvs-stdlib/src/registry.rs:1054`,
      with the returned value as a `CoreTy::Instance` per `crates/nvs-stdlib/src/instance.rs:1`.
      `properties()` counts what ordinary code can see — two of `Point`'s three
      (`examples/reflect.nvs:32`).
- [ ] **`get($object, $name)` refuses a `private` read as a `RuntimeError`** — ADR 0019's visibility
      rule, which is the half PHP's `setAccessible` gave away. The fixture's clause is deliberately
      `RuntimeError` and not `Throwable` (`examples/reflect.nvs:41`), and the conformance case is
      named at `docs/agent/loop-goal.toml:2591`.
- [ ] **`Core\Ast::parse` as inert typed data** — ADR 0052's door stays shut, so the parse answers a
      value with no path back into execution. Same registry anchor,
      `crates/nvs-stdlib/src/registry.rs:1054`, and the fixture's last two lines at
      `examples/reflect.nvs:25`.

## Backlog

- `application_code_and_the_engine_floor_produce_schema_identical_records` — M8's *Verify*, the last
  stage-7 item, over `crates/nvs-runtime/src/floor.rs` and `crates/nvs-stdlib/src/log.rs`.
- `Core\Decimal` and `Core\BigInt` — stage 8's other half, `docs/agent/loop-goal.toml:2470`.
- `nvs.toml` states neither reserve key; an operator-visible example belongs in its
  `examples/logging.nvs` `[app...log]` block, which is where `handler` already is.
- The reserve is a ceiling and nothing samples CPU time against `cpu_limit` yet — `Ctx::cpu_limit`'s
  field doc owns that gap, and it bounds the handler's time half exactly as it bounds a request's.
- `docs/agent/doc-cleanup.md`'s pass, when the user fires it.
