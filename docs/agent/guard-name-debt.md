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
2026-08-28 at `e0c9f3e`: **54 of 156 named guard tests match nothing cargo would run.** **Forty-nine**
of those have since been reconciled — the ticked lines below — leaving **5**, over the **128**
entries `tests = [...]` now holds across every `cargo-named` check (127 distinct: Stage 2 and Stage 5
both name `a_disjoint_equality_does_not_compile`). Twenty-nine of the forty-nine were a cause-2
*move* out of a `tests = [...]` list and into `cases`, so the denominator moves with them, and three
whole `[[check]]` blocks went that way entire — Stage 5's two and Stage 7's `nvs-stdlib (the
assertion roster)`.

Both numbers are derived off the tree, not carried forward: the denominator by parsing
`loop-goal.toml`, the numerator by counting the `- [ ]` lines below and confirming each one is still
a name some `tests = [...]` holds. That pass is what corrected a previous **17**, which was one short
of the eighteen lines then actually unticked.

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
2. **The check names a Rust test for work that got pinned in a `.nvst` case instead.** The commonest
   *fix*, and `nvs-stdlib (the assertion roster)` was its cleanest case: `crates/nvs-stdlib/tests/`
   holds no assertion test file at all because ADR 0079 §§ 4-5 and 22 are pinned by conformance
   cases, so all three names moved and the block itself went. **Fix: decide which tree owns the
   check, and move it** — a `kind = "nvs-suite"` entry, or a Rust test written to match. A block
   whose every name moves is deleted rather than left empty.
3. **The test is genuinely unwritten.** `every_refusal_is_a_diagnostic_or_decided` is the load-bearing
   one: it is Stage 8's "no refusal left" guard and `python tools/holes.py` still reads 17 standing
   refusal sites, so the check is red on its merits. **Fix: write it.**

A session that lands a guard test **reconciles its name here and in `loop-goal.toml` in the same
slice**. That is the only thing that keeps this file from growing back.

## The 54, by check

### Stage 3 — control and calls

`nvs-ir (control flow)`, `cargo test -p nvs-ir` — 2 of 7 unresolved

- [x] `a_do_while_lowers_its_condition_below_its_body` — cause 2, now owned by
      `tests/conformance/lang/a-do-while-runs-its-body-before-its-condition.nvst`
- [x] `a_two_representation_ternary_widens_to_one` — cause 1, now
      `a_ternary_with_mismatched_branch_types_joins_at_the_tagged_representation`
      (`crates/nvs-ir/src/lower/tests.rs`)
- [x] `a_finally_runs_when_its_catch_body_throws` — cause 2, now owned by
      `tests/conformance/error/a-finally-runs-when-its-catch-body-throws.nvst`
- [x] `an_abandoned_generator_resumes_to_unwind` — cause 1, now
      `an_abandoned_generator_resumes_into_the_finally_it_is_suspended_inside`
      (`crates/nvs-ir/src/lower/tests.rs`)
- [ ] `an_inline_producer_releases_its_value_on_the_throw_path` — cause 3, genuinely open: the
      owned-temporaries stack covers a call's arguments, a receiver and `.`/interpolation/`echo`
      operands, but a normalized subscript key and a `match` subject still release inline
- [ ] `a_transferred_argument_is_released_when_a_later_one_throws` — cause 3, genuinely open, and
      named as such in `nvs_ir::lower::Lowering`'s owned-temporaries field doc

`nvs-types (calls and loops)`, `cargo test -p nvs-types` — 0 of 6 unresolved

- [x] `a_break_past_its_nesting_is_a_compile_error` — cause 2. The check landed as
      `nvs_types::locals::check_exit_level` (`E0475`) with no Rust test naming it, so
      `tests/conformance/lang/a-break-level-that-names-no-target-is-a-diagnostic.nvst` owns it
- [x] `a_reference_array_element_is_a_compile_error` — cause 1, now
      `an_array_element_passed_by_reference_is_diagnosed`
      (`crates/nvs-types/tests/by_reference.rs:96`)

### Stage 4 — targets and mixed

`nvs-ir (targets and tags)`, `cargo test -p nvs-ir` — 1 of 8 unresolved

- [x] `a_static_property_is_an_assignment_target` — cause 2, now owned by
      `tests/conformance/class/a-static-property-is-written-and-read-through-its-class.nvst`,
      which the `nvs-suite` check already listed
- [x] `a_nested_element_write_separates_only_the_inner_array` — cause 2, now owned by the
      conformance case of the same name, already listed. `nvs-ir`'s
      `writing_through_a_nested_subscript_separates_every_level` is the write path's own shape and
      is a different claim, so it is not a rename of this one
- [x] `a_tagged_operand_dispatches_on_its_tag_for_arithmetic` — cause 2, now owned by
      `an-operator-agrees-whether-its-operand-is-typed-or-tagged.nvst` (added to the list) beside
      `a-mixed-value-answers-arithmetic-truth-and-a-subscript.nvst`
- [x] `a_tagged_operand_answers_the_truthy_table` — cause 1, now
      `a_mixed_condition_dispatches_the_truthy_table_on_the_tag`
      (`crates/nvs-ir/src/lower/tests.rs:675`)
- [x] `the_object_top_type_erases_to_the_pointer_a_class_does` — **cause 3, and now written** under
      that exact name at
      [lower/tests.rs:1369](../../crates/nvs-ir/src/lower/tests.rs#L1369). Reading the two nearest
      landed tests settled it: `a_property_access_through_a_plain_object_receiver_reads_by_name` and
      its write twin pin how an erased access *reads*, neither asserts the erasure, so this was not
      a rename. It is an agreement over the three spellings `erase_checked_ty` answers `Ty::Object`
      for — a named class, plain `object`, a shape — asserted on `Function::params` rather than in a
      snapshot, since a snapshot of one receiver cannot see the other two drift
- [ ] `an_array_conversion_walks_its_elements` — cause 3, genuinely open: `array<T> as array<U>`
      panics `nvs-ir` at `crates/nvs-ir/src/lower/expr.rs:877`, so there is nothing to guard yet
- [x] `a_tagged_operand_converts_to_bytes` — cause 2, now owned by
      `as-bytes-over-a-tagged-operand-is-decided-by-its-runtime-tag.nvst` (added to the list)

`nvs-types (targets and refusals)`, `cargo test -p nvs-types` — 0 of 7 unresolved

- [x] `a_property_default_accepts_every_compile_time_constant` — **cause 3, and now written** under
      that exact name at [tests/classes.rs:329](../../crates/nvs-types/tests/classes.rs#L329). The
      triage the item asked for came back "a fold to widen, not a test to write":
      `eval_property_default` folded a literal and `[]` and nothing else, so an enum case and
      another class's `const` — the other two members of ADR 0046 § 2's set — were `E0472`. It is
      an agreement rather than a row per form: every named constant is asserted to fold to the
      identical `ConstArg` the literal spelling of the same value folds to, with the two refusals
      the widening must not have opened (a case of the wrong enum, a `secret` constant into a
      plain slot) named at the end
- [x] `a_nullable_conversion_that_cannot_fail_is_a_compile_error` — cause 2, and the item's "reaches
      lowering as a panic today" prediction was stale: `reject_unavailable_nullable_conversion`
      ([expr/operators.rs:1296](../../crates/nvs-types/src/expr/operators.rs#L1296)) refuses it as
      `E0709` in the checker, so nothing reaches `nvs-ir` at all
- [x] `a_nullable_conversion_that_does_not_exist_is_a_compile_error` — cause 2, the other end of the
      same table (`E0708`), and **the same case owns both**:
      `tests/conformance/lang/the-nullable-conversion-table-is-closed-at-both-ends.nvst` names six
      cannot-fail rows and three no-such-row pairs in one program, which is the point — a case
      asserting one end alone cannot see the table drift open at the other.
      `classes.rs:196`'s `a_class_type_still_refuses_the_nullable_conversion` is § 3's third,
      absolute row and stays a Rust test of its own
- [x] `a_nullsafe_assignment_target_is_a_compile_error` — cause 2, not the new-diagnostic slice the
      item predicted: the refusal is landed at
      [expr/assign.rs:493](../../crates/nvs-types/src/expr/assign.rs#L493) under `E0479`, and
      `tests/conformance/class/a-nullsafe-assignment-target-is-refused.nvst` already pins both
      spellings that reach it. `crates/nvs-types/tests/` holds no test naming it, so the name moved
      to the `conformance` `cases` list rather than being renamed
- [x] `an_element_write_through_a_hooked_property_is_a_compile_error` — cause 2, the same shape as
      its nullsafe twin above: the refusal is landed at
      [expr/assign.rs:533](../../crates/nvs-types/src/expr/assign.rs#L533) and
      `tests/conformance/array/an-element-write-through-a-hooked-property-is-refused.nvst` pins all
      four subscript spellings. `by_reference.rs:119`'s
      `a_hooked_property_passed_by_reference_is_diagnosed` is a different claim — the argument
      position, not the write target — so this was not a rename of it

### Stage 5 — declared features

`nvs-types (the declared features)` — **block deleted**, 5 of 5 resolved. Every one was cause 2: the
work landed as a `.nvst` case, and `nvs-ir`'s block below went the same way, so the two `cargo-named`
checks are gone rather than renamed.

- [x] `a_property_observer_is_resolved_and_its_signature_checked` — cause 2, now
      `class/a-property-observer-sees-every-write-its-class-makes.nvst`. There is no
      observer-specific signature rule to name separately: `expr/members.rs`'s `observer_calls`
      resolves `onPropertyGet`/`onPropertySet` through the class graph, so a wrong signature is
      refused by the ordinary interface-conformance check.
- [x] `a_by_delegation_supplies_every_member_of_the_interface_it_names` — cause 2, now
      `class/a-delegate-field-must-be-able-to-answer-the-interface.nvst` (added to the list)
- [x] `an_attribute_is_retrieved_by_its_own_type` — cause 2, now
      `core/an-attribute-is-retrieved-by-the-shape-it-satisfies.nvst`
- [x] `an_ambiguous_attribute_retrieval_is_a_compile_error` — cause 2, now `E0728` in
      `reject/an-attribute-retrieval-is-refused-where-it-cannot-be-folded.nvst` (added to the list)
- [x] `an_explicit_call_site_type_argument_parses_and_checks` — cause 2, the same case: it is the one
      program where `Core\Attributes::all<T>(…)` is accepted and `get<int>(…)` refused, side by side.

`nvs-ir (the declared features)` — **block deleted**, 5 of 5 resolved, all cause 2.

- [x] `a_property_observer_runs_after_the_hook_or_the_storage` — cause 2, the observer case above
- [x] `a_delegated_call_reaches_the_object_it_names` — cause 2, now
      `class/a-delegated-interface-forwards-to-the-object-it-names.nvst`
- [x] `inline_html_at_file_scope_lowers_to_an_echo` — cause 2, now
      `lang/inline-html-at-file-scope-is-echoed-in-place.nvst`
- [x] `a_required_file_runs_its_own_top_level_statements` — cause 2, now
      `lang/a-required-file-runs-its-own-top-level-statements.nvst`, the same name as a case
- [x] `a_require_in_value_position_answers_the_files_result` — cause 2, now
      `lang/a-required-file-hands-a-value-back.nvst` (added to the list)

`nvs-stdlib (dump)`, `cargo test -p nvs-stdlib` — 2 of 2 resolved, one of each cause.

- [x] `a_dump_builds_one_diagnostic_record` — cause 1, now
      [debug.rs:525](../../crates/nvs-stdlib/src/debug.rs#L525)
      `a_dump_writes_to_the_diagnostic_channel_and_not_to_the_output`. Its own doc comment says why
      this half stays a cargo test: `--EXPECT-ERROR--` is how the runner is told a case should
      *fail*, so a suite cannot read standard error off a successful run.
- [x] `a_dump_redacts_a_secret_qualified_property` — cause 2, now
      `core/a-secret-typed-property-is-redacted-wherever-it-is-dumped.nvst`

`nvs-codegen (fatal locals)`, `cargo test -p nvs-codegen` — 1 of 1 unresolved, **cause 3**.

- [ ] `a_fatal_releases_the_frames_locals` — the work is not done. `throwing.rs` asserts the
      neighbouring half (`a_frame_that_throws_releases_the_strings_it_still_held`) and that a fatal
      is not caught, but nothing asserts what a `FATAL` does to the frame's locals, and the valgrind
      sweep cannot see it: `examples/fatal.nvs` is on its skip list for exiting non-zero by design.

### Stage 6 — checker and parser

`nvs-types (narrowing and reachability)`, `cargo test -p nvs-types` — 0 of 8 unresolved — closed

- [x] `an_instanceof_narrows_its_operand` — cause 1, now
      `an_instanceof_test_narrows_its_subject` (`crates/nvs-types/tests/narrowing.rs`)
- [x] `a_literal_comparison_narrows_its_operand` — cause 1, now
      `a_comparison_against_a_literal_narrows_its_subject` (`crates/nvs-types/tests/narrowing.rs`)
- [x] `a_match_true_arm_narrows_its_subject` — cause 2, now
      `lang/a-match-and-a-switch-over-true-narrow-per-arm.nvst` (added to the list)
- [x] `a_non_void_function_must_return_on_every_path` — was cause 3 and is now landed under its own
      name: [returns.rs:30](../../crates/nvs-types/tests/returns.rs#L30), over the `E0739` that
      `nvs_types::returns` now reports. The accepting side stays two cases
      (`lang/a-for-body-that-always-returns.nvst`, `lang/a-method-returns-early-from-inside-a-loop.nvst`).
- [x] `a_switch_and_a_try_contribute_to_definite_assignment` — cause 1, and it is two tests rather
      than one: [locals.rs:109](../../crates/nvs-types/tests/locals.rs#L109)
      `a_switch_with_default_and_a_break_in_every_case_assigns_definitely` and
      [locals.rs:170](../../crates/nvs-types/tests/locals.rs#L170)
      `a_trys_finally_assignment_reads_fine_after_it`. Both are named, so both must run.
- [x] `a_user_class_constant_has_a_type_at_an_expression_site` — cause 1, now
      [literal_types.rs:85](../../crates/nvs-types/tests/literal_types.rs#L85)
      `a_class_constant_folds_and_an_enum_case_narrows`
- [x] `a_promoted_constructor_parameter_declares_its_property` — cause 2, now
      `class/a-promoted-constructor-parameter-is-a-property.nvst` (added to the list)
- [x] `an_implicit_constructor_is_held_to_zero_arguments` — was cause 3 *in its test half only*: the
      refusal itself was already landed in `crates/nvs-types/src/expr/calls.rs`'s private
      `reject_arguments_to_implicit_constructor`, and what was owed was a test calling it. Now
      [classes.rs:136](../../crates/nvs-types/tests/classes.rs#L136), with the bare-`new` and
      inherited-constructor halves beside it.
- [x] `a_foreach_key_declared_past_string_is_a_compile_error` — cause 2, now
      `lang/a-foreach-key-binding-is-a-string-and-nothing-else.nvst` (added to the list)
- [x] `an_equality_between_incompatible_operands_is_a_compile_error` — cause 1, now
      [equality.rs:25](../../crates/nvs-types/tests/equality.rs#L25)
      `a_disjoint_equality_does_not_compile`
- [x] `a_reference_declares_the_same_type_on_both_sides` — cause 2, now
      `array/a-by-reference-binding-needs-a-variable-and-the-element-type.nvst` (added to the list)

`nvs-syntax (the last unparsed shapes)`, `cargo test -p nvs-syntax` — 0 of 4 unresolved

- [x] `a_grouped_use_parses_or_names_the_rule_that_refuses_it` — was cause 3 in both halves: the group
      form is now refused by `E0238` (`parse_use_decl`'s `recover_use_group`), and the test naming that
      rule is [parser/tests/decl.rs:330](../../crates/nvs-syntax/src/parser/tests/decl.rs#L330) under
      the guard's own name. `docs/adr/README.md` § *Decisions taken at project start* owns the rule.
- [x] `a_goto_label_is_refused_by_the_diagnostic_that_refuses_goto` — cause 1, now
      [parser/tests/stmt.rs:434](../../crates/nvs-syntax/src/parser/tests/stmt.rs#L434)
      `goto_is_diagnosed_but_still_parses`
- [x] `a_local_declared_with_a_bare_shape_type_parses` — cause 1, now
      [parser/tests/ty.rs:505](../../crates/nvs-syntax/src/parser/tests/ty.rs#L505)
      `shape_type_parses_in_every_declaration_slot`, which asserts every slot rather than the local
- [x] `an_enum_case_named_with_a_keyword_parses` — was cause 3 *in its test half only*: every keyword
      lexes at its exact lower-case spelling (ADR 0062 § 2), so a `PascalCase` case never collides with
      one and nothing had to change in the parser. Now
      [parser/tests/decl.rs:205](../../crates/nvs-syntax/src/parser/tests/decl.rs#L205), sweeping
      fifteen spellings and pinning the lower-case half against `E0220`.

### Stage 7 — testing

`nvs-types (the test table)`, `cargo test -p nvs-types` — 0 of 3 unresolved

- [x] `a_fixture_attribute_is_resolved_for_the_cases_that_name_it` — cause 1, now
      `a_fixture_attribute_builds_a_roster_keyed_by_what_it_returns`
      (`crates/nvs-types/tests/testing.rs`)
- [x] `a_test_with_attribute_expands_to_one_case_per_row` — cause 1, now
      `each_test_with_is_a_row_folded_in_parameter_order` (`crates/nvs-types/tests/testing.rs`)

`nvs-stdlib (the assertion roster)` — **the whole `[[check]]` block is gone**, all three names being
cause 2. ADR 0079's surface is landed, not owed: `#[Test]`, `#[Fixture]` and `#[TestWith]` are on
`nvs_types::derive::ATTRIBUTES` (`crates/nvs-types/src/derive.rs:77`), `Core\Test` is
`crate::test::CLASS` in `nvs_stdlib::registry`, and `crates/nvs-stdlib/src/test.rs` is the assertion
surface — so what was missing was never the work, only a Rust test naming it, and the four cases
below (plus `a-test-attribute-builds-a-table-the-runner-reports.nvst`, already listed) are in the
`nvs-suite` `cases` list instead. Each was run green before it was added.

- [x] `every_core_test_assertion_reports_through_the_one_ledger` — cause 2, now
      `core/test-three-assertions-agree-on-what-reaches-the-ledger.nvst`, which asserts § 5's ledger
      records a failure whether or not its throw is caught
- [x] `a_failed_assertion_is_catchable` — cause 2, now
      `core/a-failed-assertion-is-caught-by-name.nvst`: `Core\Test\Failure` is an ordinary
      `Throwable` a program catches, constructs and re-raises (§ 5)
- [x] `the_three_reporters_render_the_same_ledger` — cause 2, and it is three cases rather than one,
      § 22's three renderings each running the same suite:
      `lang/a-test-attribute-builds-a-table-the-runner-reports.nvst` is the human default,
      `lang/a-test-run-reports-a-versioned-json-document.nvst` and
      `lang/a-test-run-reports-the-junit-xml-ci-ingests.nvst` the two machine twins, the latter
      pinned as carrying "the same verdicts the JSON twin carries"

### Stage 8 — corpus and guards

`nvs-ir (no refusal left)`, `cargo test -p nvs-ir` — 1 of 1 unresolved, **cause 3**

- [ ] `every_refusal_is_a_diagnostic_or_decided`
