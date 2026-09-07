# Handoff

## State

**Goal 10 — stage 3 is green: a `fn` literal's parameter types come from its position, under the four
names the acceptance check lists.**

- **Nothing in stage 3 was a new feature either.** Inference, the annotated-parameter check and
  `E0450` were all landed and green; the four names were the specification for how the claims split
  across `#[test]` functions. `rule:types/callable-literal-inference` and
  `rule:types/closure-literal` are unchanged.
- **`an_unannotated_fn_parameter_takes_the_expected_types_position`** is the rename of
  `an_unannotated_parameter_takes_its_type_from_the_expected_signature`, `$n * 2` still the whole
  proof the parameter arrived as `int`.
- **The two annotated-parameter names are new bodies, not renames.** Each annotates one parameter
  beside an inferred one — `fn (int|string $n, $k): string` under `callable(int, string): string`
  and its narrower twin — so what is judged is the *literal's* annotation rather than the bare
  relation two written types already pin at `crates/nvs-types/tests/callable.rs:126`.
- **`a_block_bodied_fn_still_declares_its_return_type` asserts both sides**: the expected type gives
  a literal its parameters and never its return, so `fn ($n): int => { return $n * 2; }` is accepted
  under `callable(int): int` and `fn (int $n) => { ... }` in the same slot is `E0450`.
- **Every remaining acceptance artefact is already on disk and green.** Stages 4, 5 and 6 name six
  Rust tests, one example and two `.nvst` cases; all exist, `cargo test` is 3436 passing, and
  `examples/typed-callable.nvs` prints the `exact` check's four lines verbatim. `CoreTy::CallableTo`
  and `CoreTy::CallableShapeTo` appear nowhere in `crates/`. This session reports `DONE`; the
  driver's own acceptance run is what settles it.

Conformance 1574. Verify: 7 of 7 green.

## Next group

**Whatever the driver's acceptance names next, `docs/agent/loop-goal.toml:4679`** — file set: the
stage-4-to-6 test files, none of which this session edited. If a check is still red, its artefact
exists, so the assertion drifted rather than the name being unwritten.

- [ ] **Stage 4's six names hold as written.** `map_binds_its_result_from_a_written_fn_literal` and
      its two siblings are at `crates/nvs-types/tests/core_members.rs:81`,
      `bind_descends_into_a_callable_types_parameters` at `crates/nvs-types/src/generics.rs:521`,
      and both `nvs-stdlib` names at `crates/nvs-stdlib/src/registry.rs:3386`.
      `rule:types/callable-signature`.
- [ ] **Stage 5's three names hold as written**, at `crates/nvs-types/tests/core_members.rs:387` —
      `Task::all` rebuilding a shape from literals, from `callable`-typed variables, and no longer
      refusing a field that is not a literal. `rule:concurrency/an-all-field-answers-what-its-callable-declares`.
- [ ] **Stage 6's three codegen names hold as written**, at
      `crates/nvs-codegen/tests/closures.rs:301` — the proven site emitting no per-argument tag
      check, the bare-`callable` site still emitting it, and both metadata slots surviving.
      `rule:types/callable-variance`.

## Backlog

- Goal 10's ADR 0136 § *Revisiting* keeps whole-body return-type inference and optional/variadic
  callable parameters parked; neither is this goal's — `docs/decisions/0136.md`.
- A `callable(...)` type nested inside another callable's parameter list has no test in
  `crates/nvs-types/tests/callable.rs`; `rule:types/callable-variance` says what it should answer.
