# Handoff

## State

**Goal 6, M7, stage 6b — ADR 0006's method entry runs.** `spawn script Class::method(...)`
lowers to its own symbol (`nvs_types::CORE_SCRIPT_SPAWN_METHOD`), the child is built by
`Ctx::method_isolate` and calls `nvs_runtime::call_static`, and
`crates/nvs-cli/tests/spawn_entry.rs:120` pins it end to end: the child re-materializes a class
static from its declared default while the parent's slot is untouched, which is what makes the run
an isolate rather than a call. `E0803` is retired.

**What is left of the form is the `args:` binding, and it is refused where it is written**
(`E0804`, `crates/nvs-types/src/expr/isolate.rs:225`): a method entry that declares any parameter
does not compile, because the child calls it with no arguments and `nvs_runtime::abi::call`
requires exactly the callee's arity — a shorter list is unsound, not merely wrong. The playbook
bullet added this session is the one home of why the names for that binding exist nowhere the
runtime can reach.

**The seam widened by one value.** `Host::start_isolate` takes a `nvs_runtime::host::Entry`, whose
whole job is choosing the child's context constructor: a path entry has a unit of its own and arms
its statics from inside the program, a method entry has none. `nvs_host::Isolate` reads it through
`running_a_method_of_the_parents_unit()`.

**The acceptance check is still red and is expected to be**: three of its four tests pass and
`spawn_script_binds_args_to_the_entrys_parameters_by_name` is unwritten, which is the group below.

## Next group

**ADR 0006 § *Decision*'s `args:` binding**, over `crates/nvs-types/src/expr_table.rs`,
`crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-stdlib/src/script.rs` and
`crates/nvs-cli/tests/spawn_entry.rs`. Item 1 decides where the names come from; item 2 is the test
that closes the stage's first check.

- [ ] **The parameter names, recorded and emitted** (ADR 0006 § *Decision*) —
      `crates/nvs-types/src/expr_table.rs:82` is `ResolvedCall`, which carries `param_tys` at
      `crates/nvs-types/src/expr_table.rs:90` and no names; add them beside it and fill them where
      the struct is built, then have `crates/nvs-ir/src/lower/expr.rs:2980`'s `spawn_method_label`
      answer the label *and* the names, emitted as a fourth `ConstStr` argument on the method
      symbol at `crates/nvs-ir/src/lower/expr.rs:2888`. `nvs_runtime::MethodRow`
      (`crates/nvs-runtime/src/object.rs:453`) is not an alternative source — it has arity and tags
      only.
- [ ] **The binding itself, and `E0804` retired** (ADR 0006 § *Decision*) —
      `crates/nvs-stdlib/src/script.rs:641` is `nvs_core_script_spawn_method`, whose program
      closure calls `call_static` with an empty list; bind the crossed map's entries to the emitted
      names inside the child, after `set_isolate_argument`, and throw at the spawn for a name the
      entry does not declare or a parameter the map omits — the ADR's "reported at compile time
      when `args:` is a literal and at the spawn otherwise", whose literal half is the ADR's and
      can follow. Then delete `crates/nvs-diagnostics/src/lib.rs:3009`, its branch at
      `crates/nvs-types/src/expr/isolate.rs:225`, the fixture
      `crates/nvs-cli/tests/fixtures/spawn/method-entry-with-parameters.nvs` and the test at
      `crates/nvs-cli/tests/spawn_entry.rs:132` together.
- [ ] **The check's last test** (ADR 0006 § *Decision*) —
      `spawn_script_binds_args_to_the_entrys_parameters_by_name`, beside
      `crates/nvs-cli/tests/spawn_entry.rs:120` and through the same `run` helper at
      `crates/nvs-cli/tests/spawn_entry.rs:94`; `--config spawning.toml` is what grants
      `script.spawn`. A defaulted parameter the map omits is the case worth writing twice.

## Backlog

- A `.nvst` conformance case for the method entry — `tests/conformance/core/` holds the path
  form's four; the cli test pins the behaviour but the language's own suite does not.
- ADR 0102's route table does not cross into a method isolate: a path entry gets one from
  `nvs_cli::script`'s `program_over`, and the method form installs none — `crates/nvs-stdlib/src/script.rs:641`.
- ADR 0006 § *Revisiting*'s open question — what budget a fire-and-forget isolate spends after its
  parent is gone — is still open; `docs/adr/0072-core-task-structured-concurrency.md` § 6 says why
  `afterResponse` is not it.
