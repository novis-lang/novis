# Guard-name debt

`kind = "cargo-named"` in `loop-goal.toml` is a **substring match against the test
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
at `e0c9f3e`, **54 of 156 named guard tests matched nothing cargo would run.** All
fifty-four were reconciled, one ticked line each, and the ticked lines were then deleted in place:
`git log -S <name> -- docs/agent/guard-name-debt.md` holds every one of them with what it became.
What outlives them is the part below — why a name goes stale, and what each cause wants done.

## Why a name goes stale

Three causes, and they want different fixes. Do not assume the first one.

1. **The work landed under a different name.** The commonest. A session writes the test, names it
   the way [conventions.md](conventions.md) asks, and never reconciles `loop-goal.toml`. Confirmed
   examples: `an_instanceof_narrows_its_operand` is
   [narrowing.rs:145](../../crates/nvs-types/tests/narrowing.rs#L145)
   `an_is_test_over_a_written_class_narrows_its_subject`; `a_literal_comparison_narrows_its_operand` is
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
   holds no assertion test file at all because `rule:testing/assertions-are-typed`, `rule:testing/failure-ledger` and `rule:testing/report-formats` are pinned by conformance
   cases, so all three names moved and the block itself went. **Fix: decide which tree owns the
   check, and move it** — a `kind = "nvs-suite"` entry, or a Rust test written to match. A block
   whose every name moves is deleted rather than left empty.
3. **The test is genuinely unwritten.** `every_refusal_is_a_diagnostic_or_decided` was the
   load-bearing one: Stage 8's "no refusal left" guard, red on its merits while `python
   tools/holes.py` read seventeen standing refusal sites, and written in the end as
   [refusals.rs](../../crates/nvs-ir/tests/refusals.rs), which runs `holes.py` over the tree rather
   than carrying a second recognizer. **Fix: write it.**

A session that lands a guard test **reconciles its name here and in `loop-goal.toml` in the same
slice**. That is the only thing that keeps this file from growing back.

## Names owed on purpose are cause 3, and deliberately so

`loop-goal.toml`'s Stage 00 blocks named one test that **did not exist yet**, so
the acceptance test failed at the first check holding it and reached nothing behind it. That is the same
*symptom* as the fifty-four and it is not the same *bug*, so do not "reconcile" one of them:

| | the fifty-four | a name owed on purpose |
|---|---|---|
| the work | landed | not started |
| the fix | rewrite the name in `loop-goal.toml` | write the test |
| renaming it to something green | restores a check that was already true | hides an open hole |

**One is outstanding:**

The one before it was `every_spellable_expression_reaches_a_diagnostic_or_an_ir`, item 49's, which
landed as the second test in [type_atoms.rs](../../crates/nvs-ir/tests/type_atoms.rs) beside the type
half it is named after.

A future stage that names a test on purpose before writing it adds a bullet here, one per name, with the
item that owes it, and ends the bullet with the trailer `[until: test <name>]` that
[tools/playbook.py](../../tools/playbook.py)'s module doc defines. `python tools/playbook.py --check`
reads it, and `--retire` deletes the bullet the moment the test is in the tree — exactly as the
fifty-four ticked lines were deleted in place, and for the same reason: `git log` holds them.
