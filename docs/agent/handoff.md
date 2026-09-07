# Handoff

## State

**Goal 10 — stage 2 is green: the callable atom parses under the five names the acceptance check
lists, and two callable types compare under its other seven.**

- **Nothing in stage 2 was a new feature.** Both mechanisms were already landed and green; the
  twelve names the checks list were the specification for how the claims are split across `#[test]`
  functions, and this session split them. `rule:types/callable-signature`,
  `rule:types/callable-arity` and `rule:types/callable-variance` are unchanged.
- **The parser's three parsing claims are three tests** (`crates/nvs-syntax/src/parser/tests/ty.rs:703`):
  parameters-and-return, the empty parameter list with its `void`, and bare `callable` keeping its
  own atom. The two refusals are two more — a named parameter, and a missing return type with its
  bare-`callable` recovery.
- **Two of the seven `nvs-types` names were claims nothing asserted**:
  `a_callable_parameter_may_be_wider_than_the_slot_declares` and
  `a_callable_return_may_be_narrower_than_the_slot_declares` — the *accepting* side of each variance
  direction, which existed only as its refusal. `every_callable_signature_is_assignable_to_bare_callable`
  gained the assignment through a written-signature *variable*, since a literal alone did not make
  its name true.
- **`a_callable_return_wider_than_the_slot_is_refused` asserts the union case too**
  (`callable(int): string` filled by `fn (int $n): string|int`), because an unrelated return type
  was the only spelling the old body had and it does not make the name's claim.
- **Stage 3 is the driver's next failing check**, and three of its four names are claims already
  asserted under other names in the same file — but not names this session may reuse, since two of
  them are now spent on stage 2's variance rows. See the next group.

Conformance 1574. Verify: 7 of 7 green.

## Next group

**Stage 3's four acceptance names, `docs/agent/loop-goal.toml:4667`** — file set:
`crates/nvs-types/tests/callable.rs` alone. `rule:types/callable-literal-inference`. Stage 3's
mechanism is landed; what is missing is again the naming, plus one claim (a block body) nothing
asserts.

- [ ] **`an_unannotated_fn_parameter_takes_the_expected_types_position` exists.** Rename
      `an_unannotated_parameter_takes_its_type_from_the_expected_signature`
      (`crates/nvs-types/tests/callable.rs:159`), keeping the `$n * 2` assertion — `mixed` has no
      arithmetic, so it is the whole proof the parameter arrived as `int`.
      `rule:types/callable-literal-inference`.
- [ ] **`an_annotated_fn_parameter_may_be_wider_than_the_expected_type` and
      `an_annotated_fn_parameter_narrower_than_expected_is_refused` exist**, next to
      `crates/nvs-types/tests/callable.rs:185`. These are the *annotated* half of inference — a
      parameter the literal wrote for itself — and must not be the same two source spellings as
      `a_callable_parameter_may_be_wider_than_the_slot_declares` /
      `..._narrower_than_the_slot_is_refused` at `crates/nvs-types/tests/callable.rs:126`, which
      already pin the bare relation. Reach for a literal that annotates one parameter and infers
      another. `rule:types/callable-variance`.
- [ ] **`a_block_bodied_fn_still_declares_its_return_type` exists** at
      `crates/nvs-types/tests/callable.rs:185`. `rule:types/closure-literal` gives `fn` two bodies;
      the block-bodied one is asserted nowhere in this file, and `rule:types/callable-signature`'s
      mandatory return is what it has to keep. `E0450` is not relaxed — goal § *Standing decisions*.

## Backlog

- Stage 4: `map_binds_its_result_from_a_callable_typed_variable` and the three beside it —
  `docs/agent/loop-goal.toml:4683`.
- Stage 4: `nvs-stdlib`'s two rows checks, and `CoreTy::CallableTo` retired — ADR 0136 § *In short*.
- Stage 5: `Task::all` over a shape of `callable`-typed variables — `rule:concurrency/an-all-field-answers-what-its-callable-declares`.
- Stage 6: the tag check a proven call site stops emitting — `docs/agent/loop-goal.toml:4724`.
- `docs/rules/observability.json` and `.md` carry an uncommitted hand edit that is not this loop's;
  leave them alone.
