# Handoff

## State

**Goal 6, M7, stage 6b — ADR 0006's entry rule, half landed.** The stage's acceptance check names
four tests; `an_fn_literal_is_refused_as_a_spawn_target_naming_the_method_form` and
`a_callable_typed_variable_is_refused_as_a_spawn_target` now run and pass in
`crates/nvs-cli/tests/spawn_entry.rs`. The rule's one home is `check_entry` in
`crates/nvs-types/src/expr/isolate.rs:198`, whose own doc owns why the operand is matched on its
*shape* before it is asked about its type: a `Class::method(...)` reference and a variable holding
the callable it produces have the same type and are not the same operand.

**The method form is checked and refused, deliberately.** `E0803` says so where it is written,
under `E0777`'s reading — an ADR-specified form that is accepted and then does something else is
worse than one refused at the spawn site. Nothing below `nvs-types` lowers it, so the check's other
two tests are unwritten rather than failing. `E0803` is removed when the lowering lands, as `E0703`
was.

**Stage 6's metrics exporter is untouched and still open** — the acceptance failure outranked it,
and the previous handoff's group (ADR 0076 § 8's two dependencies, over
`crates/nvs-server/Cargo.toml:12` and `crates/nvs-server/src/metrics.rs:52`) is unchanged and still
correct. It is in Backlog below rather than lost.

**`[context] adrs` needs `0006 § Decision`.** The pack printed ADR 0006 only as a one-line ground
rule, and the whole entry rule — both forms, the named-argument binding, and the two refusals with
their reasons — is in that section. One `sed` slice fetched what the check's own name cites.

## Next group

**ADR 0006 § *Decision*'s static-method entry**, over `crates/nvs-runtime/src/script.rs`,
`crates/nvs-cli/src/script.rs`, `crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-stdlib/src/script.rs`
and `crates/nvs-types/src/expr/isolate.rs`. The first decides what the other two can be.

- [ ] **How a child isolate reaches the class its entry is declared in** (ADR 0006 § *Decision*,
      § *What is and is not shared*) — `crates/nvs-runtime/src/script.rs:69` for the `Program`
      contract and `crates/nvs-cli/src/script.rs:153` for `program_over`, which is the closure a
      method entry's own must be a sibling of. The pieces are all on disk: a `static` method
      compiles to the function named `"{class}::{method}"`
      (`crates/nvs-codegen/src/lib.rs:404`, where `build_fixture` already does exactly this),
      `Unit::function` looks it up (`crates/nvs-codegen/src/lib.rs:317`), and `Unit::install_in`
      (`crates/nvs-codegen/src/lib.rs:432`) is what arms the child's fresh statics. The open
      question is *which* unit — the class is the **parent's**, and the `Resolver` seam
      (`crates/nvs-runtime/src/script.rs:115`) is keyed by path and holds no notion of a running
      one. Recommended and not yet taken: let `install_in` also hand the `Ctx` a handle the spawn
      helper can look a function up through, so the method form needs no resolver at all and ADR
      0006's "shares nothing but compiled code" is that handle. Record it in
      `crates/nvs-runtime/src/script.rs`'s module doc, not a new ADR.
- [ ] **The lowering, the helper and the named binding** (ADR 0006 § *Decision*) —
      `crates/nvs-ir/src/lower/expr.rs:2880` is `lower_spawn_script`, whose three fixed arguments
      and `nvs_types::CORE_SCRIPT_SPAWN` symbol the method form needs a sibling of, and
      `crates/nvs-stdlib/src/script.rs:536` is `nvs_core_script_spawn`. `args:`'s entries bind to
      the entry's parameters **by name**, so the parameter list has to reach the helper: the
      checker sees it and the runtime does not. Drop the `E0803` arm in
      `crates/nvs-types/src/expr/isolate.rs:198` in the same slice.
- [ ] **The check's other two tests** (ADR 0006 § *Decision*) —
      `crates/nvs-cli/tests/spawn_entry.rs:22`'s `refusal` helper is the shape to extend with a
      `run` sibling; fixtures live beside it under `tests/fixtures/spawn/`. The names are fixed by
      `docs/agent/loop-goal.toml:3719`: `spawn_script_runs_a_static_method_reference_in_a_fresh_isolate`
      and `spawn_script_binds_args_to_the_entrys_parameters_by_name`.

## Backlog

- ADR 0076 § 8's two exporters, and what an executor-less crate can take of them —
  `crates/nvs-server/src/metrics.rs`'s § *What is not here yet* owns the gap.
- A core owns one registry and the door records each response into it — ADR 0076 §§ 1, 5.
- `crates/nvs-server/src/route.rs`'s `label` still has no caller — that file's known gap 2.
- `spawn script`'s `limits:`/`grants:`/`on:` are still refused under `E0777` — that code's own doc.
