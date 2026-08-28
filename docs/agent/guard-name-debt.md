# Guard-name debt

`kind = "cargo-named"` in [loop-goal.toml](loop-goal.toml) is a **substring match against the test
names `cargo test` actually runs**. A name that matches nothing reports `did not run`, and
`Goal.check()` returns at the first failure — so one stale name hides every check behind it.

That is not hypothetical. `a_spaceship_answers_minus_one_zero_or_one_for_a_scalar` blocked the
acceptance test at check 6 of 60 for 65 consecutive sessions: the fixtures, both `.nvst` suites, the
WSL leg and the valgrind sweep were never reached once in that time. The test it meant is
`the_spaceship_operator_is_the_compare_to_result_itself`.

**This file is a snapshot, not a source.** Re-derive it — the tree is the authority:

```
for p in nvs-syntax nvs-hir nvs-types nvs-ir nvs-runtime nvs-codegen nvs-stdlib nvs-abi-probe; do
  cargo test -p $p -- --list; done
```

then substring-match every `tests = [...]` entry in `loop-goal.toml` against that roster. Measured
2026-08-28 at `e0c9f3e`: **54 of 156 named guard tests match nothing cargo would run.**

## Why a name goes stale

Three causes, and they want different fixes. Do not assume the first one.

1. **The work landed under a different name.** The commonest. A session writes the test, names it
   the way [conventions.md](conventions.md) asks, and never reconciles `loop-goal.toml`. Confirmed
   examples: `an_instanceof_narrows_its_operand` is
   [narrowing.rs:145](../../crates/nvs-types/tests/narrowing.rs#L145)
   `an_instanceof_test_narrows_its_subject`; `a_literal_comparison_narrows_its_operand` is
   [narrowing.rs:207](../../crates/nvs-types/tests/narrowing.rs#L207)
   `a_comparison_against_a_literal_narrows_its_subject`;
   `a_fixture_attribute_is_resolved_for_the_cases_that_name_it` is
   [testing.rs:94](../../crates/nvs-types/tests/testing.rs#L94)
   `a_fixture_attribute_builds_a_roster_keyed_by_what_it_returns`;
   `a_test_with_attribute_expands_to_one_case_per_row` is
   [testing.rs:155](../../crates/nvs-types/tests/testing.rs#L155)
   `each_test_with_is_a_row_folded_in_parameter_order`;
   `an_abandoned_generator_resumes_to_unwind` is
   `an_abandoned_generator_resumes_into_the_finally_it_is_suspended_inside`.
   **Fix: rewrite the name in `loop-goal.toml`.** Nothing else.
2. **The check names a Rust test for work that got pinned in a `.nvst` case instead.** `nvs-stdlib
   (the assertion roster)` is the clear one: `crates/nvs-stdlib/tests/` holds no assertion test file
   at all, because ADR 0079 § 4's roster was pinned by conformance cases. **Fix: decide which tree
   owns the check, and move it** — a `kind = "nvs-suite"` entry, or a Rust test written to match.
3. **The test is genuinely unwritten.** `every_refusal_is_a_diagnostic_or_decided` is the load-bearing
   one: it is Stage 8's "no refusal left" guard and `python tools/holes.py` still reads 17 standing
   refusal sites, so the check is red on its merits. **Fix: write it.**

A session that lands a guard test **reconciles its name here and in `loop-goal.toml` in the same
slice**. That is the only thing that keeps this file from growing back.

## The 54, by check

### Stage 3 — control and calls

`nvs-ir (control flow)`, `cargo test -p nvs-ir` — 6 of 7 unresolved

- [ ] `a_do_while_lowers_its_condition_below_its_body`
- [ ] `a_two_representation_ternary_widens_to_one`
- [ ] `a_finally_runs_when_its_catch_body_throws`
- [ ] `an_abandoned_generator_resumes_to_unwind` — cause 1, name above
- [ ] `an_inline_producer_releases_its_value_on_the_throw_path`
- [ ] `a_transferred_argument_is_released_when_a_later_one_throws`

`nvs-types (calls and loops)`, `cargo test -p nvs-types` — 2 of 6 unresolved

- [ ] `a_break_past_its_nesting_is_a_compile_error`
- [ ] `a_reference_array_element_is_a_compile_error` — likely
      `an_array_element_passed_by_reference_is_diagnosed`

### Stage 4 — targets and mixed

`nvs-ir (targets and tags)`, `cargo test -p nvs-ir` — 7 of 8 unresolved

- [ ] `a_static_property_is_an_assignment_target`
- [ ] `a_nested_element_write_separates_only_the_inner_array`
- [ ] `a_tagged_operand_dispatches_on_its_tag_for_arithmetic`
- [ ] `a_tagged_operand_answers_the_truthy_table`
- [ ] `the_object_top_type_erases_to_the_pointer_a_class_does`
- [ ] `an_array_conversion_walks_its_elements`
- [ ] `a_tagged_operand_converts_to_bytes`

`nvs-types (targets and refusals)`, `cargo test -p nvs-types` — 5 of 11 unresolved

- [ ] `a_property_default_accepts_every_compile_time_constant`
- [ ] `a_nullable_conversion_that_cannot_fail_is_a_compile_error`
- [ ] `a_nullable_conversion_that_does_not_exist_is_a_compile_error`
- [ ] `a_nullsafe_assignment_target_is_a_compile_error`
- [ ] `an_element_write_through_a_hooked_property_is_a_compile_error`

### Stage 5 — declared features

`nvs-types (the declared features)`, `cargo test -p nvs-types` — 5 of 5 unresolved

- [ ] `a_property_observer_is_resolved_and_its_signature_checked`
- [ ] `a_by_delegation_supplies_every_member_of_the_interface_it_names`
- [ ] `an_attribute_is_retrieved_by_its_own_type`
- [ ] `an_ambiguous_attribute_retrieval_is_a_compile_error`
- [ ] `an_explicit_call_site_type_argument_parses_and_checks`

`nvs-ir (the declared features)`, `cargo test -p nvs-ir` — 5 of 5 unresolved

- [ ] `a_property_observer_runs_after_the_hook_or_the_storage`
- [ ] `a_delegated_call_reaches_the_object_it_names`
- [ ] `inline_html_at_file_scope_lowers_to_an_echo`
- [ ] `a_required_file_runs_its_own_top_level_statements`
- [ ] `a_require_in_value_position_answers_the_files_result`

`nvs-stdlib (dump)`, `cargo test -p nvs-stdlib` — 2 of 2 unresolved

- [ ] `a_dump_builds_one_diagnostic_record`
- [ ] `a_dump_redacts_a_secret_qualified_property`

`nvs-codegen (fatal locals)`, `cargo test -p nvs-codegen` — 1 of 1 unresolved

- [ ] `a_fatal_releases_the_frames_locals`

### Stage 6 — checker and parser

`nvs-types (narrowing and reachability)`, `cargo test -p nvs-types` — 11 of 11 unresolved

- [ ] `an_instanceof_narrows_its_operand` — cause 1, name above
- [ ] `a_literal_comparison_narrows_its_operand` — cause 1, name above
- [ ] `a_match_true_arm_narrows_its_subject`
- [ ] `a_non_void_function_must_return_on_every_path`
- [ ] `a_switch_and_a_try_contribute_to_definite_assignment`
- [ ] `a_user_class_constant_has_a_type_at_an_expression_site`
- [ ] `a_promoted_constructor_parameter_declares_its_property`
- [ ] `an_implicit_constructor_is_held_to_zero_arguments` — likely
      `reject_arguments_to_implicit_constructor`
- [ ] `a_foreach_key_declared_past_string_is_a_compile_error`
- [ ] `an_equality_between_incompatible_operands_is_a_compile_error`
- [ ] `a_reference_declares_the_same_type_on_both_sides`

`nvs-syntax (the last unparsed shapes)`, `cargo test -p nvs-syntax` — 4 of 4 unresolved

- [ ] `a_grouped_use_parses_or_names_the_rule_that_refuses_it`
- [ ] `a_goto_label_is_refused_by_the_diagnostic_that_refuses_goto`
- [ ] `a_local_declared_with_a_bare_shape_type_parses`
- [ ] `an_enum_case_named_with_a_keyword_parses`

### Stage 7 — testing

`nvs-types (the test table)`, `cargo test -p nvs-types` — 2 of 3 unresolved

- [ ] `a_fixture_attribute_is_resolved_for_the_cases_that_name_it` — cause 1, name above
- [ ] `a_test_with_attribute_expands_to_one_case_per_row` — cause 1, name above

`nvs-stdlib (the assertion roster)`, `cargo test -p nvs-stdlib` — 3 of 3 unresolved, **cause 2**

- [ ] `every_core_test_assertion_reports_through_the_one_ledger`
- [ ] `a_failed_assertion_is_catchable`
- [ ] `the_three_reporters_render_the_same_ledger`

### Stage 8 — corpus and guards

`nvs-ir (no refusal left)`, `cargo test -p nvs-ir` — 1 of 1 unresolved, **cause 3**

- [ ] `every_refusal_is_a_diagnostic_or_decided`
