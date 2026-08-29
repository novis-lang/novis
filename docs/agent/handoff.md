# Handoff

## State

**Stage 5 is closed but for one item that has nothing to attach to yet.** ADR 0023 § 2's graph
copy is `crates/nvs-runtime/src/graph.rs` — one walk, two carriers — `Core\Serialize::encode`/
`decode` are the member pair over it, and the checker half now refuses both directions. All
four of Stage 5's `cargo-named` tests are green and `examples/serialize.nvs` prints its four
frozen lines. What remains of the stage is joining the **live** carrier to the `spawn`
boundary, which Stage 6 has to build first.

**ADR 0033 § 4's cross-boundary sink is one check for both carriers**, by design:
`nvs_types::expr::quals::reject_secret_boundary_argument` (`E0775`) reads the written
arguments of a call, so when `spawn`/`spawn worker`/`spawn script` lower they pass their own
argument list to it rather than growing a second rule. It is a call-site walk and not a
parameter type because `encode` declares `mixed`, which a `secret string` satisfies. The
complement is `graph.rs`'s `field_is_secret`: a `secret`-typed *property* is invisible from a
call site, so the walk refuses that one at run time.

**`Core\Secret::reveal()` does not exist**, and four diagnostics' help text already names it as
the way out. It is not a missing row: the registry has no parameter spelling that *accepts* a
qualifier, and `Qual::Launder` has no consumer in `nvs-types` either — the same reason
`Qual::Sink` needs none (see the playbook bullet). Item 18's escape hatch is open at both ends
and is a design slice, not a member slice.

**The driver's failing acceptance check is Stage 6's**, not a regression: `examples/isolate.nvs`
exits on `E0703 — 'spawn script' is not compiled yet`, which is item 20's whole subject.

**Orientation gaps.** `[context] adrs` names ADR 0023 § 2 only; § 3 and ADR 0033 § 4 were both
read by hand this session and § 4 is the one that specifies the sink. `[context] modules` still
has no pattern for `nvs-types/src/expr/` — `quals.rs`, `calls.rs` and `core_lib.rs` are where
every qualifier rule lives — nor for `nvs-stdlib/src/instance.rs` or `nvs-runtime/src/object.rs`.

## Next group

**Stage 6's opening: `spawn script` gets something to lower to.** File set:
`docs/adr/` (the pre-authorized isolate slot), `crates/nvs-host/` and
`crates/nvs-ir/src/lower/expr.rs:444` (the `E0703` refusal this group replaces), against
`crates/nvs-runtime/src/ctx.rs`'s `Ctx::child` and `crates/nvs-host/src/group.rs` (the child
context and the one `Host` implementor, both already landed).

- [ ] **ADR 0116 — the isolate heap boundary.** Goal item 20 and one of the goal's two
      pre-authorized ADR slots: what an arena is, what it costs, and how a value crosses —
      the parts [ADR 0006](../adr/0006-isolated-script-execution.md) states as behaviour
      rather than as implementation. Re-derive the next free number before creating the file.
      `crates/nvs-runtime/src/graph.rs`'s `Live` carrier is the crossing this specifies.
- [ ] **The `Isolate` type in `crates/nvs-host`**, item 20's implementation: its own arena,
      `Core` accessor backing state and config overlay. It is a task with a heap boundary, so
      it sits beside `group.rs`'s `SchedulerHost` rather than inside it.
- [ ] **`spawn script` lowers**, replacing `crates/nvs-ir/src/lower/expr.rs:444`'s `E0703`.
      That closes the acceptance check `examples/isolate.nvs` currently fails on; the
      `ScriptResult` shape and the value-crossing refusals are items 22-23 and come after.

## Backlog

- `Core\Secret::reveal()` — goal item 18's escape hatch, needing a qualifier-accepting parameter
  spelling and a `Qual::Launder` consumer. `docs/agent/goals/2-concurrency.md:118`.
- Stage 5's live-carrier join: `graph.rs`'s `Live` reached from the `spawn` boundary once one
  exists. `docs/agent/goals/2-concurrency.md:110`.
- Items 21-24: the request tree's shared budget, `Core\Script::args()`, the crossing refusals,
  and `output: capture|inherit`. `docs/agent/goals/2-concurrency.md:131`.
- M4's 1000-case corpus count, met as the suite grows. `docs/implementation-plan.md`.
- `python tools/check-migration.py` at 34%, the parity program's own measure.
