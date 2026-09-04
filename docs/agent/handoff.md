# Handoff

## State

**Goal 6, M7, stage 6b — ADR 0006's entry rule, half landed, and the method form's runtime seam
now under it.** `check_entry` (`crates/nvs-types/src/expr/isolate.rs:198`) refuses an `fn` literal
and a `callable` variable (`E0802`) and refuses the *specified* method form (`E0803`) because
nothing lowers it; both refusal tests in `crates/nvs-cli/tests/spawn_entry.rs` pass.

**What landed this session is the answer to "which unit".** A `Class::method` isolate runs code the
*parent's* unit already holds, so it needs no resolver and no path: the class table already crossed
(`Ctx::isolate` clones the `ErrorClass` handle, which is what `call_static` resolves a label
through), and the missing half was the unit's static-property recipes. `Ctx::install_statics` now
takes an `Rc<[Option<FieldDefault>]>` and keeps it, so a context that was armed can arm a child
against the same slot numbering with no `Unit` in hand; `Ctx::method_isolate`
(`crates/nvs-runtime/src/ctx/isolate.rs:222`) is that one call.
`crates/nvs-runtime/src/script.rs`'s module doc is the **one home** of why the method form never
reaches the `Resolver` seam, and of the capability reading — `Cap::ScriptSpawn` asked with
`Scope::Unscoped`, since a method has no path to prefix-check but `script.spawn = false` must still
refuse both forms.

**The acceptance check is still red and is expected to be**: its other two tests
(`spawn_script_runs_a_static_method_reference_in_a_fresh_isolate`,
`spawn_script_binds_args_to_the_entrys_parameters_by_name`) are unwritten, not failing. `E0803` is
removed by the group below, as `E0703` was.

## Next group

**ADR 0006 § *Decision*'s method entry, end to end**, over `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-stdlib/src/script.rs`, `crates/nvs-types/src/expr/isolate.rs` and
`crates/nvs-cli/tests/spawn_entry.rs`. Item 1 decides the IR shape the other two are written
against; the runtime side it calls into is already on disk and tested.

- [ ] **The lowering and the spawn helper** (ADR 0006 § *Decision*) —
      `crates/nvs-ir/src/lower/expr.rs:2880` is `lower_spawn_script`, which funnels both forms into
      one `InstKind::CoreCall { symbol: nvs_types::CORE_SCRIPT_SPAWN, args: [path, args, output] }`.
      The method form differs in the first argument only: lower the operand as a **constant label**
      `"{class}::{method}"` rather than as an expression, and select a helper that builds the child
      with `Ctx::method_isolate` (`crates/nvs-runtime/src/ctx/isolate.rs:222`) and calls
      `nvs_runtime::call_static` (`crates/nvs-runtime/src/dispatch.rs:128`) instead of asking
      `nvs_runtime::script::resolve` (`crates/nvs-runtime/src/script.rs:231`). A second `CORE_*`
      symbol beside `CORE_SCRIPT_SPAWN` is the cheaper split than a fourth argument nothing but the
      helper reads. Drop the `E0803` branch at `crates/nvs-types/src/expr/isolate.rs:210` and retire
      the code at `crates/nvs-diagnostics/src/lib.rs:3008` in the same slice.
- [ ] **`args:` bound to the entry's parameters by name** (ADR 0006 § *Decision*) — the open design
      call, and it is a *data* question: `crates/nvs-runtime/src/object.rs:453`'s `MethodRow` carries
      `arity` and `param_tags` and **no parameter names**, so a runtime binder has nothing to bind
      against. Two ways out, and the ADR's own sentence ("at compile time when `args:` is a literal
      and at the spawn otherwise") wants both: the compiler orders a literal `args:` into the
      declared parameter order at `crates/nvs-ir/src/lower/expr.rs:2893` where the option is already
      lowered, and a dynamic `args:` needs names on the row or in a side table. Landing the literal
      half first is honest and keeps `E0803` off the dynamic one.
- [ ] **The check's other two tests** (ADR 0006 § *Decision*) —
      `crates/nvs-cli/tests/spawn_entry.rs:1`, beside the two refusals that already run there:
      `spawn_script_runs_a_static_method_reference_in_a_fresh_isolate` and
      `spawn_script_binds_args_to_the_entrys_parameters_by_name`, which are the names
      `docs/agent/loop-goal.toml:3719` asks for.

## Backlog

- Stage 6's metrics exporter, ADR 0076 § 8's two dependencies — `crates/nvs-server/Cargo.toml:12`
  and `crates/nvs-server/src/metrics.rs:52`. Untouched, still correct, outranked twice now.
- **`[context] adrs` still needs `0006 § Decision`.** The pack prints ADR 0006 only as a one-line
  ground rule, and the whole entry rule — both forms, the named binding, the two refusals — is in
  that section. A second session has now sliced it by hand.
- `[context] modules` prints no `nvs-ir` or `nvs-codegen` line, so the next group's own files have
  no map entry; one `nvs-ir/src/lower/*` pattern would close it.
